use std::{
    collections::{HashMap, HashSet},
    fs,
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
    #[pyo3(get)]
    pub install_missing_packages: Option<bool>,

    // @TODO: More fields, like:
    //  - Enforce transitive packages
    //  - Install types-packages

    #[serde(skip)]
    maps: Option<Maps>,

    #[serde(skip)]
    installed: HashSet<NormalizedPkgName>,
}

impl Config {
    pub fn load(start_dir: &Path) -> anyhow::Result<Self> {
        if let Some(config_path) = find_pyproject(start_dir) {
            let contents = fs::read_to_string(config_path)?;
            let pyproject: PyProject = toml::from_str(contents.as_str())?;
            let config = pyproject.tool.and_then(|tool| tool.impending);
            if let Some(config) = config {
                return Ok(config);
            }
        }
        Err(anyhow::anyhow!("Expected a tool.impending section"))
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
        if let Some(module_map) = &self.module_map {
            if let Some(pkgname) = module_map.get(&modname) {
                return Some(pep508_normalize(pkgname));
            }
        }

        // FALLBACK!
        if let Some(maps) = &self.maps {
            let reqmap = &maps.reqmap;

            // Attempt: 1:1 matching
            if reqmap.contains_key(&modname) {
                return Some(modname);
            }

            // Attempt: py{modname}
            //  E.g. pygithub, pypng
            let pkgname = format!("py{}", modname);
            if reqmap.contains_key(&pkgname) {
                return Some(pkgname);
            }

            // Attempt: python-{modname}
            //  E.g. python-dateutil, python-dotenv
            let pkgname = format!("python_{}", modname);
            if reqmap.contains_key(&pkgname) {
                return Some(pkgname);
            }

            // @TODO: More strategies:
            //  - Azure does azure_NAME where NAME is "replace dots with underscores"
            //      e.g. azure_mgmt_datalake_analytics -> azure.mgmt.datalake.analytics
        }

        // At least we tried
        None
    }
}

#[pymethods]
impl Config {
    // @TODO: I think this could probably be handled better in Rust.
    //  E.g. does the distribution finding, can look things up transitively, etc...
    pub fn get_expected_version(&mut self, fullname: String) -> anyhow::Result<Option<String>> {
        self.initialize_maps()?;
        let pkgname = self.find_package(fullname);
        if pkgname.is_none() {
            return Ok(None);
        }
        let pkgname = pkgname.unwrap();

        if let Some(maps) = &self.maps {
            if let Some(reqinfo) = maps.reqmap.get(&pkgname) {
                return Ok(reqinfo.pinned_version());
            }
        }
        Ok(None)
    }

    // @TODO: Special-case impending? :)
    pub fn maybe_install(&mut self, fullname: String) -> anyhow::Result<bool> {
        self.initialize_maps()?;

        let pkgname = self.find_package(fullname);
        if pkgname.is_none() {
            return Ok(false);
        }
        let pkgname = pkgname.unwrap();

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
                .status()
                .expect("failed to execute process");

            // @TODO: Can I drain the RHS for more perf?
            self.installed.extend(seen);
            return Ok(true);
        }

        Ok(false)
    }
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
