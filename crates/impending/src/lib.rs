use anyhow::Context;
use pyo3::prelude::*;
use std::env;
use std::path::{Path, PathBuf};

mod mpf;
mod venv_utils;

fn find_caller_dir(py: Python<'_>) -> PyResult<PathBuf> {
    let inspect = py.import("inspect")?;
    let caller_filename = inspect
        .getattr("stack")?
        .call0()?
        .get_item(1)?
        .getattr("filename")?
        .extract::<String>()?;
    Ok(Path::new(&caller_filename).parent().unwrap().to_path_buf())
}

#[pyfunction]
fn install(py: Python<'_>) -> PyResult<()> {
    if let Ok(no_install) = env::var("IMPENDING_NO_INSTALL") {
        if no_install == "1" {
            // @TODO: Log...
            return Ok(());
        }
    }

    let sys = py.import("sys")?;
    if venv_utils::venv_path(sys)?.is_none() {
        // @TODO: Error
        return Ok(());
    }

    let start_dir = find_caller_dir(py).context("Failed to get the caller's directory")?;
    let config = impending_config::Config::load(&start_dir)
    .context("Failed to load the relevant pyproject.toml")?;

let our_mpf = mpf::ImpendingMPF::from_config(config)?;
sys.getattr("meta_path")?
.call_method("insert", (0, our_mpf), None)?;

Ok(())
}

#[pymodule]
#[pyo3(name = "_impl")]
fn _impl(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(install, m)?)?;
    Ok(())
}
