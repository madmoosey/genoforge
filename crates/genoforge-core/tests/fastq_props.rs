//! Property tests for the FASTQ reader and accumulator.

#![allow(clippy::cast_possible_truncation)] // u64 <-> usize on test-sized inputs

use std::io::Cursor;

use genoforge_core::fastq::{stats_from_reader, QcAccumulator, Reader, Record, StatsOptions};
use proptest::prelude::*;

fn record_strategy() -> impl Strategy<Value = Record> {
    (0usize..120).prop_flat_map(|len| {
        (
            "[A-Za-z0-9_ ]{1,12}",
            prop::collection::vec(prop::sample::select(b"ACGTNacgtn".to_vec()), len),
            prop::collection::vec(b'!'..=b'~', len),
        )
            .prop_map(|(name, seq, qual)| Record {
                name: name.into_bytes(),
                seq,
                qual,
            })
    })
}

fn records_strategy() -> impl Strategy<Value = Vec<Record>> {
    prop::collection::vec(record_strategy(), 0..40)
}

fn to_fastq(records: &[Record]) -> Vec<u8> {
    let mut out = Vec::new();
    for r in records {
        out.push(b'@');
        out.extend_from_slice(&r.name);
        out.push(b'\n');
        out.extend_from_slice(&r.seq);
        out.extend_from_slice(b"\n+\n");
        out.extend_from_slice(&r.qual);
        out.push(b'\n');
    }
    out
}

proptest! {
    /// Serialising records and parsing them back is the identity.
    #[test]
    fn parse_round_trip(records in records_strategy()) {
        let bytes = to_fastq(&records);
        let parsed: Vec<Record> = Reader::new(Cursor::new(bytes))
            .records()
            .collect::<Result<_, _>>()
            .unwrap();
        prop_assert_eq!(parsed, records);
    }

    /// Summary statistics obey their definitions regardless of input.
    #[test]
    fn stats_invariants(records in records_strategy()) {
        let bytes = to_fastq(&records);
        let s = stats_from_reader(Reader::new(Cursor::new(bytes)), StatsOptions::default()).unwrap();
        let total_bases: usize = records.iter().map(Record::len).sum();
        prop_assert_eq!(s.total_reads as usize, records.len());
        prop_assert_eq!(s.total_bases as usize, total_bases);
        prop_assert_eq!(s.max_len as usize, records.iter().map(Record::len).max().unwrap_or(0));
        prop_assert_eq!(s.min_len as usize, records.iter().map(Record::len).min().unwrap_or(0));
        for (name, v) in [
            ("gc", s.gc_frac), ("n", s.n_frac), ("q20", s.q20_frac),
            ("q30", s.q30_frac), ("dup", s.dup_frac_est),
        ] {
            prop_assert!((0.0..=1.0).contains(&v), "{} = {}", name, v);
        }
        prop_assert!(s.q30_frac <= s.q20_frac);
        prop_assert_eq!(s.per_position_mean_qual.len() as u64, s.max_len);
    }

    /// Splitting a stream anywhere and merging the halves equals one pass,
    /// even with a sketch far smaller than the number of distinct reads.
    #[test]
    fn merge_is_exact(records in records_strategy(), split in 0usize..40, k in 1usize..8) {
        let split = split.min(records.len());
        let mut all = QcAccumulator::with_sketch_size(k);
        let mut left = QcAccumulator::with_sketch_size(k);
        let mut right = QcAccumulator::with_sketch_size(k);
        for (i, r) in records.iter().enumerate() {
            all.add(r);
            if i < split { left.add(r) } else { right.add(r) }
        }
        left.merge(&right);
        prop_assert_eq!(all.finish(), left.finish());
    }

    /// The parallel driver with tiny batches agrees with a single accumulator.
    #[test]
    fn driver_matches_accumulator(records in records_strategy(), batch in 1usize..7) {
        let mut acc = QcAccumulator::with_sketch_size(16);
        for r in &records {
            acc.add(r);
        }
        let bytes = to_fastq(&records);
        let s = stats_from_reader(
            Reader::new(Cursor::new(bytes)),
            StatsOptions { threads: 2, batch_records: batch, sketch_size: 16 },
        )
        .unwrap();
        prop_assert_eq!(s, acc.finish());
    }
}
