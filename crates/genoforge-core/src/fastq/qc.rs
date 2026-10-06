//! Per-file QC accumulator and the summary it produces.

use serde::{Deserialize, Serialize};

use super::dedup::{fnv1a, BottomK};
use super::reader::{Record, QUAL_OFFSET};
use crate::frac;

/// Phred threshold for the `q20_frac` metric.
const Q20: u64 = 20;
/// Phred threshold for the `q30_frac` metric.
const Q30: u64 = 30;

/// Running QC counters for a stream of FASTQ records.
///
/// Accumulators are independent and mergeable, which is what lets the driver
/// fold a file across threads without sharing state. All counters are exact;
/// only `dup_frac_est` is an estimate (see [`BottomK`]).
#[derive(Debug, Clone)]
pub struct QcAccumulator {
    reads: u64,
    bases: u64,
    min_len: u64,
    max_len: u64,
    gc: u64,
    acgt: u64,
    n: u64,
    qual_sum: u64,
    q20: u64,
    q30: u64,
    pos_qual_sum: Vec<u64>,
    pos_count: Vec<u64>,
    sketch: BottomK,
}

impl Default for QcAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl QcAccumulator {
    /// Default bottom-k sketch size for the duplicate-rate estimate.
    pub const DEFAULT_SKETCH_SIZE: usize = 10_000;

    /// Accumulator with the default sketch size.
    #[must_use]
    pub fn new() -> Self {
        Self::with_sketch_size(Self::DEFAULT_SKETCH_SIZE)
    }

    /// Accumulator with an explicit sketch size (larger = tighter estimate,
    /// more memory; `0` is treated as `1`).
    #[must_use]
    pub fn with_sketch_size(sketch_size: usize) -> Self {
        Self {
            reads: 0,
            bases: 0,
            min_len: u64::MAX,
            max_len: 0,
            gc: 0,
            acgt: 0,
            n: 0,
            qual_sum: 0,
            q20: 0,
            q30: 0,
            pos_qual_sum: Vec::new(),
            pos_count: Vec::new(),
            sketch: BottomK::new(sketch_size),
        }
    }

    /// Fold one record in.
    ///
    /// Quality bytes below the Phred+33 offset count as Phred 0; the reader
    /// never produces them, but a hand-built [`Record`] might.
    pub fn add(&mut self, rec: &Record) {
        let len = rec.seq.len() as u64;
        self.reads += 1;
        self.bases += len;
        self.min_len = self.min_len.min(len);
        self.max_len = self.max_len.max(len);

        for &b in &rec.seq {
            match b {
                b'G' | b'g' | b'C' | b'c' => {
                    self.gc += 1;
                    self.acgt += 1;
                }
                b'A' | b'a' | b'T' | b't' => self.acgt += 1,
                b'N' | b'n' => self.n += 1,
                _ => {}
            }
        }

        if self.pos_qual_sum.len() < rec.qual.len() {
            self.pos_qual_sum.resize(rec.qual.len(), 0);
            self.pos_count.resize(rec.qual.len(), 0);
        }
        for (i, &q) in rec.qual.iter().enumerate() {
            let phred = u64::from(q.saturating_sub(QUAL_OFFSET));
            self.qual_sum += phred;
            if phred >= Q20 {
                self.q20 += 1;
            }
            if phred >= Q30 {
                self.q30 += 1;
            }
            self.pos_qual_sum[i] += phred;
            self.pos_count[i] += 1;
        }

        self.sketch.insert(fnv1a(&rec.seq));
    }

    /// Fold another accumulator in. Equivalent to having added its records here.
    pub fn merge(&mut self, other: &Self) {
        self.reads += other.reads;
        self.bases += other.bases;
        self.min_len = self.min_len.min(other.min_len);
        self.max_len = self.max_len.max(other.max_len);
        self.gc += other.gc;
        self.acgt += other.acgt;
        self.n += other.n;
        self.qual_sum += other.qual_sum;
        self.q20 += other.q20;
        self.q30 += other.q30;
        if self.pos_qual_sum.len() < other.pos_qual_sum.len() {
            self.pos_qual_sum.resize(other.pos_qual_sum.len(), 0);
            self.pos_count.resize(other.pos_count.len(), 0);
        }
        for (i, (&s, &c)) in other.pos_qual_sum.iter().zip(&other.pos_count).enumerate() {
            self.pos_qual_sum[i] += s;
            self.pos_count[i] += c;
        }
        self.sketch.merge(&other.sketch);
    }

    /// Records folded in so far.
    #[must_use]
    pub fn reads(&self) -> u64 {
        self.reads
    }

    /// Produce the summary. Cheap; the accumulator stays usable.
    #[must_use]
    pub fn finish(&self) -> FastqStats {
        FastqStats {
            total_reads: self.reads,
            total_bases: self.bases,
            min_len: if self.reads == 0 { 0 } else { self.min_len },
            max_len: self.max_len,
            mean_len: frac(self.bases, self.reads),
            mean_qual: frac(self.qual_sum, self.bases),
            gc_frac: frac(self.gc, self.acgt),
            n_frac: frac(self.n, self.bases),
            q20_frac: frac(self.q20, self.bases),
            q30_frac: frac(self.q30, self.bases),
            dup_frac_est: self.sketch.duplicate_fraction(),
            per_position_mean_qual: self
                .pos_qual_sum
                .iter()
                .zip(&self.pos_count)
                .map(|(&s, &c)| frac(s, c))
                .collect(),
        }
    }
}

/// QC summary for one FASTQ file.
///
/// Fractions are in `[0, 1]`. `mean_qual` is the mean per-base Phred score
/// (not the error-probability mean). `dup_frac_est` is the estimated fraction
/// of reads whose sequence is an exact repeat of an earlier read, from a
/// bottom-k sketch; for paired-end data run each file separately.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FastqStats {
    /// Number of records.
    pub total_reads: u64,
    /// Sum of read lengths.
    pub total_bases: u64,
    /// Shortest read (0 for an empty file).
    pub min_len: u64,
    /// Longest read.
    pub max_len: u64,
    /// `total_bases / total_reads`.
    pub mean_len: f64,
    /// Mean Phred score over all bases.
    pub mean_qual: f64,
    /// G+C over A+C+G+T (N excluded).
    pub gc_frac: f64,
    /// N over all bases.
    pub n_frac: f64,
    /// Fraction of bases with Phred >= 20.
    pub q20_frac: f64,
    /// Fraction of bases with Phred >= 30.
    pub q30_frac: f64,
    /// Estimated duplicate-read fraction.
    pub dup_frac_est: f64,
    /// Mean Phred score at each read position (length = longest read).
    pub per_position_mean_qual: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(seq: &[u8], qual: &[u8]) -> Record {
        Record {
            name: b"r".to_vec(),
            seq: seq.to_vec(),
            qual: qual.to_vec(),
        }
    }

    #[test]
    fn empty_accumulator_is_all_zero() {
        let s = QcAccumulator::new().finish();
        assert_eq!(s.total_reads, 0);
        assert_eq!(s.min_len, 0);
        assert_eq!(s.max_len, 0);
        assert_eq!(s.mean_len, 0.0);
        assert_eq!(s.dup_frac_est, 0.0);
        assert_eq!(s.per_position_mean_qual, Vec::<f64>::new());
    }

    #[test]
    fn counts_bases_quality_and_positions() {
        let mut acc = QcAccumulator::new();
        acc.add(&rec(b"ACGTN", b"IIII!")); // I = Phred 40, ! = Phred 0
        acc.add(&rec(b"GG", b"55")); // 5 = Phred 20
        let s = acc.finish();
        assert_eq!(s.total_reads, 2);
        assert_eq!(s.total_bases, 7);
        assert_eq!((s.min_len, s.max_len), (2, 5));
        assert_eq!(s.mean_len, 3.5);
        assert_eq!(s.gc_frac, 4.0 / 6.0);
        assert_eq!(s.n_frac, 1.0 / 7.0);
        assert_eq!(s.mean_qual, 200.0 / 7.0);
        assert_eq!(s.q20_frac, 6.0 / 7.0);
        assert_eq!(s.q30_frac, 4.0 / 7.0);
        assert_eq!(s.per_position_mean_qual, vec![30.0, 30.0, 40.0, 40.0, 0.0]);
        assert_eq!(s.dup_frac_est, 0.0);
    }

    #[test]
    fn duplicate_estimate_counts_repeated_sequences() {
        let mut acc = QcAccumulator::new();
        for _ in 0..3 {
            acc.add(&rec(b"ACGT", b"IIII"));
        }
        acc.add(&rec(b"TTTT", b"IIII"));
        assert_eq!(acc.finish().dup_frac_est, 0.5);
    }

    #[test]
    fn merge_equals_sequential_add() {
        let records = [
            rec(b"ACGTN", b"IIII!"),
            rec(b"GG", b"55"),
            rec(b"ACGTN", b"IIII!"),
            rec(b"", b""),
        ];
        let mut all = QcAccumulator::with_sketch_size(2);
        let mut left = QcAccumulator::with_sketch_size(2);
        let mut right = QcAccumulator::with_sketch_size(2);
        for (i, r) in records.iter().enumerate() {
            all.add(r);
            if i < 2 {
                left.add(r);
            } else {
                right.add(r);
            }
        }
        left.merge(&right);
        assert_eq!(all.finish(), left.finish());
    }
}
