//! Python bindings for `genoforge-core`, built with maturin into `genoforge._native`.
//!
//! Rules for this crate:
//! - Hot loops detach from the interpreter with `py.detach` (releases the GIL)
//!   so a Django worker or a notebook stays responsive while Rust crunches a file.
//! - Core errors map to Python exceptions at this boundary and nowhere else.
//! - Every public function has a matching signature in `python/genoforge/_native.pyi`.

use pyo3::prelude::*;

/// Version of the compiled Rust core.
#[pyfunction]
fn core_version() -> &'static str {
    genoforge_core::version()
}

/// GC fraction of a nucleotide sequence (`bytes`), ignoring N and non-ACGT.
///
/// The slice borrows the Python buffer directly (no copy).
#[pyfunction]
fn gc_fraction(py: Python<'_>, sequence: &[u8]) -> f64 {
    py.detach(|| genoforge_core::gc_fraction(sequence))
}

/// The `genoforge._native` module.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(core_version, m)?)?;
    m.add_function(wrap_pyfunction!(gc_fraction, m)?)?;
    Ok(())
}
