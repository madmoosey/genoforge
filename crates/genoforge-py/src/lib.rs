//! Python bindings for `genoforge-core`, built with maturin into `genoforge._native`.
//!
//! Rules for this crate:
//! - Hot loops detach from the interpreter with `py.detach` (releases the GIL)
//!   so a Django worker or a notebook stays responsive while Rust crunches a file.
//! - Core errors map to Python exceptions at this boundary and nowhere else.
//! - Every public item has a matching signature in `python/genoforge/_native.pyi`.

use std::io::ErrorKind;
use std::path::PathBuf;

use genoforge_core::fastq::{self, FastqStats, StatsOptions};
use pyo3::exceptions::{PyFileNotFoundError, PyOSError, PyPermissionError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Newtype so a core error can be turned into a `PyErr` (orphan rule).
struct CoreError(genoforge_core::Error);

impl From<CoreError> for PyErr {
    fn from(CoreError(e): CoreError) -> PyErr {
        use genoforge_core::Error as E;
        let msg = e.to_string();
        match e {
            E::Io { source, .. } => match source.kind() {
                ErrorKind::NotFound => PyFileNotFoundError::new_err(msg),
                ErrorKind::PermissionDenied => PyPermissionError::new_err(msg),
                _ => PyOSError::new_err(msg),
            },
            E::Read(_) => PyOSError::new_err(msg),
            E::Parse { .. } | E::InvalidArgument(_) => PyValueError::new_err(msg),
        }
    }
}

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

/// QC summary for one FASTQ file. Immutable; attributes mirror the Rust struct.
#[pyclass(frozen, get_all, module = "genoforge._native", name = "FastqStats")]
struct PyFastqStats {
    total_reads: u64,
    total_bases: u64,
    min_len: u64,
    max_len: u64,
    mean_len: f64,
    mean_qual: f64,
    gc_frac: f64,
    n_frac: f64,
    q20_frac: f64,
    q30_frac: f64,
    dup_frac_est: f64,
    per_position_mean_qual: Vec<f64>,
}

impl From<FastqStats> for PyFastqStats {
    fn from(s: FastqStats) -> Self {
        Self {
            total_reads: s.total_reads,
            total_bases: s.total_bases,
            min_len: s.min_len,
            max_len: s.max_len,
            mean_len: s.mean_len,
            mean_qual: s.mean_qual,
            gc_frac: s.gc_frac,
            n_frac: s.n_frac,
            q20_frac: s.q20_frac,
            q30_frac: s.q30_frac,
            dup_frac_est: s.dup_frac_est,
            per_position_mean_qual: s.per_position_mean_qual,
        }
    }
}

#[pymethods]
impl PyFastqStats {
    /// Plain-dict form, e.g. for JSON serialisation or a Django `JSONField`.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        d.set_item("total_reads", self.total_reads)?;
        d.set_item("total_bases", self.total_bases)?;
        d.set_item("min_len", self.min_len)?;
        d.set_item("max_len", self.max_len)?;
        d.set_item("mean_len", self.mean_len)?;
        d.set_item("mean_qual", self.mean_qual)?;
        d.set_item("gc_frac", self.gc_frac)?;
        d.set_item("n_frac", self.n_frac)?;
        d.set_item("q20_frac", self.q20_frac)?;
        d.set_item("q30_frac", self.q30_frac)?;
        d.set_item("dup_frac_est", self.dup_frac_est)?;
        d.set_item("per_position_mean_qual", &self.per_position_mean_qual)?;
        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!(
            "FastqStats(total_reads={}, total_bases={}, mean_len={:.1}, mean_qual={:.2}, \
             gc_frac={:.3}, q30_frac={:.3}, dup_frac_est={:.3})",
            self.total_reads,
            self.total_bases,
            self.mean_len,
            self.mean_qual,
            self.gc_frac,
            self.q30_frac,
            self.dup_frac_est,
        )
    }
}

/// Streaming QC statistics for a FASTQ file (plain or gzip, sniffed by content).
///
/// Releases the GIL for the whole computation. `threads=0` uses one worker per
/// core. Raises `FileNotFoundError` / `PermissionError` / `OSError` for I/O
/// problems and `ValueError` (with the 1-based record index) for malformed input.
#[pyfunction]
#[pyo3(signature = (path, *, threads = 0, batch_records = 50_000, sketch_size = 10_000))]
fn fastq_stats(
    py: Python<'_>,
    path: PathBuf,
    threads: usize,
    batch_records: usize,
    sketch_size: usize,
) -> PyResult<PyFastqStats> {
    let opts = StatsOptions {
        threads,
        batch_records,
        sketch_size,
    };
    let stats = py
        .detach(|| fastq::stats_from_path(path, opts))
        .map_err(CoreError)?;
    Ok(stats.into())
}

/// The `genoforge._native` module.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(core_version, m)?)?;
    m.add_function(wrap_pyfunction!(gc_fraction, m)?)?;
    m.add_function(wrap_pyfunction!(fastq_stats, m)?)?;
    m.add_class::<PyFastqStats>()?;
    Ok(())
}
