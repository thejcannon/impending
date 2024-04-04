use impending_config::Config;
use pyo3::prelude::*;
use std::env;
use std::process::Command;
use std::collections::HashSet;

use anyhow::Result;
use impending_module_map::{stdlib::STDLIB_MODULES, ModuleMap};
use impending_requirements::{parse_requirements_txt, DepMap, ReqMap};

use crate::venv_utils;

#[pyclass]
pub struct ImpendingMPF {
    pub config: Option<Config>,
    pub reqmap: Option<ReqMap>,
    pub depmap: Option<DepMap>,
    pub module_map: ModuleMap,
}

impl ImpendingMPF {
    pub fn from_config(config: Option<Config>) -> Result<Self> {
        let mut reqmap = None;
        let mut depmap = None;
        if let Some(lockfile) = config.as_ref().and_then(|c| c.lockfile.clone()) {
            let parsed = parse_requirements_txt(&lockfile)?;
            reqmap = Some(parsed.0);
            depmap = Some(parsed.1);
        }
        let module_map = ModuleMap::new(
            reqmap
                .as_ref()
                .map(|reqmap| reqmap.keys().cloned().into_iter()),
        );
        Ok(ImpendingMPF {
            config,
            reqmap,
            depmap,
            module_map,
        })
    }

    fn _find_spec(
        self_: &PyRef<Self>,
        py: Python,
        sys: &PyModule,
        fullname: &String,
        path: &PyAny,
        target: &PyAny,
    ) -> PyResult<Option<PyObject>> {
        let self_py = self_.into_py(py);
        for finder in sys.getattr("meta_path")?.iter()? {
            if let Ok(finder) = finder {
                if !finder.is(&self_py) {
                    let spec = finder.call_method("find_spec", (fullname, path, target), None)?;
                    if !spec.is_none() {
                        return PyResult::Ok(Some(spec.into()));
                    }
                }
            }
        }
        Ok(None)
    }

    fn maybe_install_reqs(&self, sys: &PyModule, pkgname: String) -> anyhow::Result<bool> {
        let mut requirements = vec![];
        let mut packages = vec![pkgname];
        let mut seen: HashSet<String> = HashSet::new();
        while let Some(pkgname) = packages.pop() {
            if !seen.insert(pkgname.clone()) {
                continue;
            }

            if let Some(depmap) = &self.depmap {
                if let Some(dependencies) = depmap.get(&pkgname) {
                    packages.extend(dependencies.iter().cloned());
                }
            }
            if let Some(reqmap) = &self.reqmap {
                if let Some(reqinfo) = reqmap.get(&pkgname) {
                    requirements.push(reqinfo.to_requirement(&pkgname));
                }
            }
        }

        if !requirements.is_empty() {
            let mut args = self
                .config
                .as_ref()
                .and_then(|config| config.installer_cmd.clone())
                .unwrap_or(vec![
                    "pip".to_owned(),
                    "install".to_owned(),
                    "-qq".to_owned(),
                ]);
            // @TODO: Verify the config is nonempty
            let program = args.remove(0);
            args.extend(requirements);
            Command::new(program)
                .args(args)
                .env(
                    "PATH",
                    format!(
                        "{}/bin:{}",
                        venv_utils::venv_path(sys)?.unwrap(),
                        env::var("PATH").unwrap_or_default()
                    ),
                )
                .status()
                .expect("failed to execute process");
            return Ok(true);
        }

        Ok(false)
    }
}

#[pymethods]
impl ImpendingMPF {
    fn find_spec(
        self_: PyRef<Self>,
        py: Python,
        fullname: String,
        path: &PyAny,
        target: &PyAny,
    ) -> PyResult<Option<PyObject>> {
        let sys = py.import("sys")?;

        if let Some(spec) = ImpendingMPF::_find_spec(&self_, py, sys, &fullname, path, target)? {
            return Ok(Some(spec.into()));
        }

        if STDLIB_MODULES.contains(fullname.as_str()) {
            return Ok(None);
        }

        let pkgname = self_.module_map.find_package(fullname.clone())?;
        if let Some(pkgname) = pkgname {
            if self_.maybe_install_reqs(sys, pkgname)? {
                return ImpendingMPF::_find_spec(&self_, py, sys, &fullname, path, target);
            }
        }

        Ok(None)
    }
}
