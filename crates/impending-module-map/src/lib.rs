use anyhow::Result;
use std::collections::HashMap;

pub mod stdlib;

pub struct ModuleMap {
    _impl: HashMap<String, String>,
}

impl ModuleMap {
    pub fn new<I>(pkg_names: Option<I>) -> Self
    where
        I: Iterator<Item = String>,
    {
        return ModuleMap {
            _impl: pkg_names
                .map(|names| {
                    HashMap::from_iter(
                        // @TODO: Actual normalization
                        names.map(|name| (name.replace("-", "_"), name)),
                    )
                })
                .unwrap_or(HashMap::new()),
        };
    }

    pub fn find_package(&self, modname: String) -> Result<Option<String>> {
        // Attempt: 1:1 matching
        if let Some(pkgname) = self._impl.get(&modname) {
            return Ok(Some(pkgname.clone()));
        }

        // Attempt: py{modname}
        //  E.g. pygithub, pypng
        if let Some(pkgname) = self._impl.get(format!("py{}", modname).as_str()) {
            return Ok(Some(pkgname.clone()));
        }

        // Attempt: python-{modname}
        //  E.g. python-dateutil, python-dotenv
        if let Some(pkgname) = self._impl.get(format!("python_{}", modname).as_str()) {
            return Ok(Some(pkgname.clone()));
        }

        // At least we tried
        Ok(None)
    }
}
