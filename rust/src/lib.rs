pub mod grid;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes, PyString};
use crate::grid::{make_cells, FullGrid};

#[pyclass(module = "viewfinder_core._native")]
#[derive(Debug)]
pub struct RayTracer {
    grid: FullGrid
}

impl RayTracer {

}

#[pymethods]
impl RayTracer {
    #[new]
    // #[pyo3(signature = (capacity = 1024))]
    fn py_new(path: String) -> Self {
        Self {
            grid: make_cells() // TODO
        }
    }
}

/// Native core of viewfinder (Rust, via PyO3).
// Declarative (inline) module: required for `experimental-inspect` stub generation.
#[pymodule]
mod _native {
    #[pymodule_export]
    use super::{RayTracer};

    #[allow(non_upper_case_globals)]
    #[pymodule_export]
    const __version__: &str = env!("CARGO_PKG_VERSION");
}

#[cfg(test)]
mod tests {
    use super::*;


}
