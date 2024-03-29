use std::env;
use std::path::Path;

use pyo3::types::PyModule;

pub fn venv_path(sys: &PyModule) -> anyhow::Result<Option<String>> {
    if let Ok(virtual_env) = env::var("VIRTUAL_ENV") {
        return Ok(Some(virtual_env));
    }

    let sys_prefix = sys.getattr("prefix")?.extract::<String>()?;
    let venv_marker = Path::new(&sys_prefix).join("pyvenv.cfg");

    if venv_marker.exists() {
        return Ok(Some(sys_prefix));
    }

    Ok(None)
}
