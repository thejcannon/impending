use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use impending_requirements::{
    parse_requirements_txt, utils::pep508_normalize, Maps, NormalizedPkgName,
};
use pyo3::{pyclass, pymethods};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PyProject {
    pub tool: Option<PyProjectTool>,
}

#[derive(Deserialize)]
pub struct PyProjectTool {
    pub impending: Option<Config>,
}

#[pyclass(module = "impending")]
#[derive(Deserialize)]
pub struct Config {
    pub lockfile: Option<String>,
    pub installer_cmd: Option<Vec<String>>,
    pub module_map: Option<HashMap<String, String>>,
    pub package_companions: Option<HashMap<String, Vec<String>>>,

    // Toggles
    #[pyo3(get)]
    pub enforce_package_versions: Option<bool>,
    // @TODO: Maybe this is just on by default for lockfiles?
    #[pyo3(get)]
    pub enforce_transitive_package_versions: Option<bool>,
    #[pyo3(get)]
    pub install_missing_packages: Option<bool>,
    // @TODO: More fields, like:
    //  - Install types-packages
    #[serde(skip)]
    maps: Option<Maps>,

    #[serde(skip)]
    installed: HashSet<NormalizedPkgName>,
}

impl Config {
    pub fn load(start_dir: &Path) -> anyhow::Result<Self> {
        if let Some(config_path) = find_pyproject(start_dir) {
            let contents = fs::read_to_string(config_path.clone())?;
            let pyproject: PyProject = toml::from_str(contents.as_str())?;
            let config = pyproject.tool.and_then(|tool| tool.impending);
            if let Some(config) = config {
                return Ok(config);
            }
            return Err(anyhow::anyhow!(
                "Failed to load tool.impending section in {:?}.",
                config_path
            ));
        }
        Err(anyhow::anyhow!(
            "Couldn't find pyproject.toml at or above {:?}",
            start_dir
        ))
    }

    fn initialize_maps(&mut self) -> anyhow::Result<()> {
        if self.maps.is_none() {
            if let Some(lockfile) = &self.lockfile {
                self.maps = Some(parse_requirements_txt(&lockfile)?);
            }
        }
        Ok(())
    }

    fn find_package(&self, modname: String) -> Option<NormalizedPkgName> {
        // @TODO: If the modname is the prefix of a namespace package, either explictly
        //  via the user, or implcitly via fallback, we should signal that.

        if let Some(module_map) = &self.module_map {
            if let Some(pkgname) = module_map.get(&modname) {
                return Some(pep508_normalize(pkgname));
            }
        }

        // Ok, so we weren't explictly told what this maps to, that's ok.
        // Let's try a robust set of fallback(s).
        // (inspired by https://joshcannon.me/2024/07/05/package-names.html)
        if let Some(maps) = &self.maps {
            let normalized = pep508_normalize(&modname);
            let reqmap = &maps.reqmap;

            let transformations = [
                |s: &String| s.to_string(),
                |s: &String| format!("django-{}", s),
                |s: &String| format!("python-{}", s),
                |s: &String| format!("py{}", s),
                |s: &String| format!("{}-py", s),
                |s: &String| format!("{}-python", s),
            ];

            return transformations.iter()
                .map(|transform| transform(&normalized))
                .find(|pkgname| reqmap.contains_key(pkgname));
        }

        None  // At least we tried
    }
}

#[pymethods]
impl Config {
    pub fn get_package_name(&mut self, fullname: String) -> anyhow::Result<Option<String>> {
        self.initialize_maps()?;
        Ok(self.find_package(fullname))
    }

    pub fn get_expected_version(&mut self, pkgname: String) -> anyhow::Result<Option<String>> {
        self.initialize_maps()?;
        if let Some(maps) = &self.maps {
            if let Some(reqinfo) = maps.reqmap.get(&pkgname) {
                return Ok(reqinfo.pinned_version());
            }
        }
        Ok(None)
    }

    pub fn maybe_install_package(&mut self, sys_prefix: String, pkgname: String) -> anyhow::Result<bool> {
        let mut requirements = vec![];
        let mut packages = vec![pkgname];
        let mut seen: HashSet<NormalizedPkgName> = HashSet::new();
        while let Some(pkgname) = packages.pop() {
            if self.installed.contains(&pkgname) || !seen.insert(pkgname.clone()) {
                continue;
            }

            if let Some(maps) = &self.maps {
                if let Some(reqinfo) = maps.reqmap.get(&pkgname) {
                    requirements.push(reqinfo.to_requirement(&pkgname));
                }
                if let Some(dependencies) = maps.depmap.get(&pkgname) {
                    packages.extend(dependencies.iter().cloned());
                }
            }

            if let Some(package_companions) = &self.package_companions {
                if let Some(companions) = package_companions.get(&pkgname) {
                    packages.extend(companions.iter().map(|name| pep508_normalize(name)));
                }
            }
        }

        if !requirements.is_empty() {
            let mut args = self.installer_cmd.clone().unwrap_or(vec![
                "pip".to_owned(),
                "install".to_owned(),
                "-qq".to_owned(),
            ]);

            let program = args.remove(0);
            args.extend(requirements);
            Command::new(program)
                .args(args)
                .env(
                    "PATH",
                    format!("{}/bin:{}", sys_prefix, env::var("PATH").unwrap()),
                )
                .status()
                .expect("failed to execute process");

            // @TODO: Can I drain the RHS for more perf?
            self.installed.extend(seen);
            return Ok(true);
        }

        Ok(false)
    }

    // @TODO: Special-case impending? :)
}

fn find_pyproject(mut current: &Path) -> Option<PathBuf> {
    loop {
        let pyproject = current.join("pyproject.toml");
        if pyproject.exists() {
            return Some(pyproject);
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}
