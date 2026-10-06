//! Core genomics parsing, QC and statistics for genoforge.
//!
//! This crate has no knowledge of Python. Everything here is plain Rust with
//! `Result`-based error handling so it can be driven from the CLI, from the
//! `PyO3` bindings in `genoforge-py`, or from tests, identically.
//!
//! Module map:
//! - [`fastq`]  streaming FASTQ reader, QC accumulator, parallel driver
//! - `kmer`     2-bit packed k-mers (planned)
//! - `bam`      BAM/CRAM mapping statistics (planned)
//! - `vcf`      VCF record model and statistics (planned)
//! - `simulate` seeded read simulator with planted SNVs (planned)

pub mod error;
pub mod fastq;

pub use error::{Error, Result};

/// Crate version as compiled in, so every layer (CLI, Python, API) can report
/// which core it is running against.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Fraction of G/C bases in `seq`, ignoring `N`/`n` and any non-ACGT bytes.
///
/// Returns `0.0` for a sequence with no A/C/G/T bases at all.
#[must_use]
pub fn gc_fraction(seq: &[u8]) -> f64 {
    let mut gc = 0u64;
    let mut acgt = 0u64;
    for &b in seq {
        match b {
            b'G' | b'g' | b'C' | b'c' => {
                gc += 1;
                acgt += 1;
            }
            b'A' | b'a' | b'T' | b't' => acgt += 1,
            _ => {}
        }
    }
    frac(gc, acgt)
}

/// `num / den` as a float, or `0.0` when `den` is zero.
///
/// Precision loss only matters past 2^53, which no per-file counter reaches.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub(crate) fn frac(num: u64, den: u64) -> f64 {
    if den == 0 {
        0.0
    } else {
        num as f64 / den as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn gc_fraction_counts_only_acgt() {
        assert!((gc_fraction(b"GGCC") - 1.0).abs() < f64::EPSILON);
        assert!((gc_fraction(b"ATAT") - 0.0).abs() < f64::EPSILON);
        assert!((gc_fraction(b"GCNNAT") - 0.5).abs() < f64::EPSILON);
        assert!((gc_fraction(b"gcat") - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn gc_fraction_empty_and_all_n_is_zero() {
        assert!((gc_fraction(b"") - 0.0).abs() < f64::EPSILON);
        assert!((gc_fraction(b"NNNN") - 0.0).abs() < f64::EPSILON);
    }
}
