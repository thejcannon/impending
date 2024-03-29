use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct PyProject {
    pub tool: Option<PyProjectTool>,
}

#[derive(Deserialize)]
pub struct PyProjectTool {
    pub impending: Option<Config>,
}

#[derive(Deserialize)]
pub struct Config {
    pub lockfile: Option<String>,
    pub installer_cmd: Option<Vec<String>>,
}

impl Config {
    pub fn load(start_dir: &Path) -> anyhow::Result<Option<Self>> {
        if let Some(config_path) = find_pyproject(start_dir) {
            let contents = fs::read_to_string(config_path)?;
            let pyproject: PyProject = toml::from_str(contents.as_str())?;
            return Ok(pyproject.tool.and_then(|tool| tool.impending));
        }
        Ok(None)
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
