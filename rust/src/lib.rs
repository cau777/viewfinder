pub mod grid;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes, PyString};

#[pyclass(module = "viewfinder_core._native")]
#[derive(Debug)]
pub struct RayTracer {
    data: Vec<u8>,
    capacity: usize,
}

impl RayTracer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { data: Vec::with_capacity(capacity), capacity }
    }

    /// Append bytes, refusing to grow past `capacity`.
    pub fn extend(&mut self, bytes: &[u8]) -> Result<usize, String> {
        let new_len = self.data.len() + bytes.len();
        if new_len > self.capacity {
            return Err(format!(
                "buffer overflow: {} + {} bytes exceeds capacity {}",
                self.data.len(),
                bytes.len(),
                self.capacity
            ));
        }
        self.data.extend_from_slice(bytes);
        Ok(self.data.len())
    }

    /// Adler-32 checksum of the buffer contents.
    pub fn checksum(&self) -> u32 {
        const MOD: u32 = 65_521;
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in &self.data {
            a = (a + byte as u32) % MOD;
            b = (b + a) % MOD;
        }
        (b << 16) | a
    }

    /// Most frequent bytes, as (byte, count) sorted by count desc then byte asc.
    pub fn histogram_top(&self, n: usize) -> Vec<(u8, usize)> {
        let mut counts = [0usize; 256];
        for &byte in &self.data {
            counts[byte as usize] += 1;
        }
        let mut pairs: Vec<(u8, usize)> = counts
            .iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .map(|(b, &c)| (b as u8, c))
            .collect();
        pairs.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
        pairs.truncate(n);
        pairs
    }

    pub fn hex_preview(&self, n: usize) -> String {
        self.data.iter().take(n).map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
    }
}

#[pymethods]
impl RayTracer {
    #[new]
    // #[pyo3(signature = (capacity = 1024))]
    fn py_new(path: String) -> Self {

    }

    /// Append a Python object to the buffer and return the new length.
    ///
    /// Accepts `str` (UTF-8 encoded), `bytes`, `bytearray`, or any iterable
    /// of ints in 0..=255 (e.g. `list[int]`, `memoryview`).
    // Explicit annotation: PyO3 can't infer a type for `&Bound<PyAny>`. Builtins only,
    // because the stub generator copies the string verbatim without adding imports.
    #[pyo3(signature = (chunk: "str | bytes | bytearray | list[int]"))]
    fn push(&mut self, chunk: &Bound<'_, PyAny>) -> PyResult<usize> {
        let bytes: Vec<u8> = if let Ok(s) = chunk.cast::<PyString>() {
            s.to_str()?.as_bytes().to_vec()
        } else if let Ok(b) = chunk.cast::<PyBytes>() {
            b.as_bytes().to_vec()
        } else if let Ok(ba) = chunk.cast::<PyByteArray>() {
            ba.to_vec()
        } else {
            let mut out = Vec::new();
            for item in chunk.try_iter().map_err(|_| {
                PyTypeError::new_err(format!(
                    "push() expects str, bytes, bytearray or an iterable of ints, got {}",
                    chunk.get_type().name().map(|n| n.to_string()).unwrap_or_default()
                ))
            })? {
                out.push(item?.extract::<u8>().map_err(|_| {
                    PyValueError::new_err("iterable items must be ints in range 0..=255")
                })?);
            }
            out
        };
        self.extend(&bytes).map_err(PyValueError::new_err)
    }

    /// Describe the buffer contents. `preview` caps how many bytes appear in `hex_preview`.
    #[pyo3(signature = (preview = 16))]
    fn summary(&self, preview: usize) -> Summary {
        Summary {
            length: self.data.len(),
            capacity: self.capacity,
            checksum: self.checksum(),
            hex_preview: self.hex_preview(preview),
            histogram_top: self.histogram_top(5),
        }
    }

    /// Copy the buffer out as Python `bytes`.
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    fn clear(&mut self) {
        self.data.clear();
    }

    #[getter]
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn __len__(&self) -> usize {
        self.data.len()
    }

    fn __repr__(&self) -> String {
        format!("ByteBuffer(len={}, capacity={})", self.data.len(), self.capacity)
    }
}

/// Native core of viewfinder (Rust, via PyO3).
// Declarative (inline) module: required for `experimental-inspect` stub generation.
#[pymodule]
mod _native {
    #[pymodule_export]
    use super::{RayTracer, Summary};

    #[allow(non_upper_case_globals)]
    #[pymodule_export]
    const __version__: &str = env!("CARGO_PKG_VERSION");
}

#[cfg(test)]
mod tests {
    use super::*;


}
