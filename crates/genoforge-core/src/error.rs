//! Error type shared by every module in the core crate.

use std::path::PathBuf;

/// All errors the core library can produce.
///
/// Library code returns these; it never panics on bad input. The CLI and the
/// Python bindings decide how to surface them (exit codes, Python exceptions).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file could not be opened.
    #[error("cannot open {path}: {source}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A read from an already-open stream failed (disk error, bad gzip, ...).
    #[error("read error: {0}")]
    Read(#[source] std::io::Error),

    /// Input did not conform to the expected file format.
    #[error("{format} parse error at record {record}: {message}")]
    Parse {
        /// Format name, e.g. `FASTQ`.
        format: &'static str,
        /// 1-based record index where the problem was detected.
        record: u64,
        /// Human-readable detail.
        message: String,
    },

    /// A caller-supplied parameter was out of range.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
