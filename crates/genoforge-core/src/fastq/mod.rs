//! FASTQ: streaming reader, QC accumulator and a parallel statistics driver.
//!
//! The reader never holds more than one record in memory. The driver reads
//! records sequentially into a reusable batch, folds the batch across a rayon
//! pool with one [`QcAccumulator`] per task, and merges the partials. Nothing
//! in the hot loop takes a lock.
//!
//! ```no_run
//! use genoforge_core::fastq::{stats_from_path, StatsOptions};
//!
//! let stats = stats_from_path("reads.fastq.gz", StatsOptions::default())?;
//! println!("{} reads, Q30 = {:.3}", stats.total_reads, stats.q30_frac);
//! # Ok::<(), genoforge_core::Error>(())
//! ```

mod dedup;
mod qc;
mod reader;

use std::io::BufRead;
use std::path::Path;

use rayon::prelude::*;

use crate::{Error, Result};

pub use qc::{FastqStats, QcAccumulator};
pub use reader::{Reader, Record, Records, QUAL_OFFSET};

/// Tuning knobs for [`stats_from_path`] / [`stats_from_reader`].
#[derive(Debug, Clone, Copy)]
pub struct StatsOptions {
    /// Worker threads. `0` uses rayon's global pool (one thread per core).
    pub threads: usize,
    /// Records read before handing a batch to the pool. Bounds peak memory.
    pub batch_records: usize,
    /// Size of the bottom-k sketch used for the duplicate-rate estimate.
    pub sketch_size: usize,
}

impl Default for StatsOptions {
    fn default() -> Self {
        Self {
            threads: 0,
            batch_records: 50_000,
            sketch_size: QcAccumulator::DEFAULT_SKETCH_SIZE,
        }
    }
}

/// Compute QC statistics for a FASTQ file, plain or gzip-compressed.
///
/// # Errors
/// Returns [`Error::Io`] if the file cannot be opened, [`Error::Read`] for a
/// failure mid-stream, and [`Error::Parse`] with the 1-based record index for
/// malformed input.
pub fn stats_from_path(path: impl AsRef<Path>, opts: StatsOptions) -> Result<FastqStats> {
    stats_from_reader(Reader::from_path(path)?, opts)
}

/// Compute QC statistics from an already-open [`Reader`].
///
/// # Errors
/// As [`stats_from_path`], minus the open error. Also returns
/// [`Error::InvalidArgument`] if a dedicated thread pool cannot be created.
pub fn stats_from_reader<R: BufRead>(
    mut reader: Reader<R>,
    opts: StatsOptions,
) -> Result<FastqStats> {
    let batch_records = opts.batch_records.max(1);
    let pool = if opts.threads > 0 {
        Some(
            rayon::ThreadPoolBuilder::new()
                .num_threads(opts.threads)
                .build()
                .map_err(|e| Error::InvalidArgument(e.to_string()))?,
        )
    } else {
        None
    };

    let mut batch: Vec<Record> = Vec::with_capacity(batch_records.min(1 << 16));
    let mut total = QcAccumulator::with_sketch_size(opts.sketch_size);
    loop {
        let n = fill_batch(&mut reader, &mut batch, batch_records)?;
        if n == 0 {
            break;
        }
        let records = &batch[..n];
        let fold = || reduce_batch(records, opts.sketch_size);
        let partial = match &pool {
            Some(p) => p.install(fold),
            None => fold(),
        };
        total.merge(&partial);
    }
    Ok(total.finish())
}

/// Read up to `cap` records into `batch`, reusing existing allocations.
/// Returns how many records are valid in `batch[..n]`.
fn fill_batch<R: BufRead>(
    reader: &mut Reader<R>,
    batch: &mut Vec<Record>,
    cap: usize,
) -> Result<usize> {
    let mut n = 0;
    while n < cap {
        if n == batch.len() {
            batch.push(Record::default());
        }
        if !reader.read_into(&mut batch[n])? {
            break;
        }
        n += 1;
    }
    Ok(n)
}

/// Fold a batch across the current rayon pool.
fn reduce_batch(records: &[Record], sketch_size: usize) -> QcAccumulator {
    /// Records per task: large enough to amortise the per-task accumulator.
    const CHUNK: usize = 2_048;
    records
        .par_chunks(CHUNK)
        .map(|chunk| {
            let mut acc = QcAccumulator::with_sketch_size(sketch_size);
            for rec in chunk {
                acc.add(rec);
            }
            acc
        })
        .reduce(
            || QcAccumulator::with_sketch_size(sketch_size),
            |mut a, b| {
                a.merge(&b);
                a
            },
        )
}
