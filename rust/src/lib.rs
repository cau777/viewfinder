use pyo3::prelude::*;

/// A Python module implemented in Rust.
// #[pymodule]
// mod _native {
    #[pyclass] pub struct ByteBuffer { data: Vec<u8>, capacity: usize }   // Rust owns the data

    #[pymethods] impl ByteBuffer {
        #[new] #[pyo3(signature = (capacity = 1024))]
        fn py_new(capacity: usize) -> Self {
            Self{data: Vec::with_capacity(capacity), capacity}
        }                        // Python constructor
    
        #[pyo3(signature = (chunk: "str | bytes | bytearray | list[int]"))]
        fn push(&mut self, chunk: &Bound<'_, PyAny>) -> PyResult<usize> {
            todo!()
        }
    }
    
    #[pymodule] mod _native {
        #[pymodule_export] use super::{ByteBuffer};

        #[allow(non_upper_case_globals)]
        #[pymodule_export]
        const __version__: &str = env!("CARGO_PKG_VERSION");
    }
// }
