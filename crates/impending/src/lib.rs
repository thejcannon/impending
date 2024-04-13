use anyhow::Context;
use pyo3::prelude::*;
use std::env;
use std::path::PathBuf;

#[pyfunction]
fn load_config(start_dir: Option<String>) -> PyResult<impending_config::Config> {
    let start_dir = start_dir
        .map(|s| PathBuf::from(s).parent().unwrap().to_path_buf())
        .unwrap_or(env::current_dir()?);
    let config = impending_config::Config::load(&start_dir)
        .context("Failed to load the relevant pyproject.toml")?;

    Ok(config)
}

#[pymodule]
#[pyo3(name = "_impl")]
fn _impl(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(load_config, m)?)?;
    Ok(())
}
