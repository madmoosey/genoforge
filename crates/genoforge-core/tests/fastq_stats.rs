//! End-to-end checks of the FASTQ driver against a hand-computed fixture.

use std::io::{Cursor, Write};
use std::path::PathBuf;

use flate2::{write::GzEncoder, Compression};
use genoforge_core::fastq::{stats_from_path, stats_from_reader, FastqStats, Reader, StatsOptions};
use genoforge_core::Error;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny.fastq")
}

fn assert_close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() < 1e-9, "{what}: {a} != {b}");
}

/// Values worked out by hand from `tests/fixtures/tiny.fastq`:
/// r1/r3 = 10 bases Q40 (5 GC), r2 = 10 bases Q20 (8 GC, 2 N), r4 = 2 bases Q0.
fn check(stats: &FastqStats) {
    assert_eq!(stats.total_reads, 4);
    assert_eq!(stats.total_bases, 32);
    assert_eq!(stats.min_len, 2);
    assert_eq!(stats.max_len, 10);
    assert_close(stats.mean_len, 8.0, "mean_len");
    assert_close(stats.mean_qual, 1000.0 / 32.0, "mean_qual");
    assert_close(stats.gc_frac, 18.0 / 30.0, "gc_frac");
    assert_close(stats.n_frac, 2.0 / 32.0, "n_frac");
    assert_close(stats.q20_frac, 30.0 / 32.0, "q20_frac");
    assert_close(stats.q30_frac, 20.0 / 32.0, "q30_frac");
    assert_close(stats.dup_frac_est, 0.25, "dup_frac_est");
    assert_eq!(stats.per_position_mean_qual.len(), 10);
    assert_close(stats.per_position_mean_qual[0], 25.0, "pos 0");
    assert_close(stats.per_position_mean_qual[1], 25.0, "pos 1");
    for (i, &q) in stats.per_position_mean_qual.iter().enumerate().skip(2) {
        assert_close(q, 100.0 / 3.0, &format!("pos {i}"));
    }
}

#[test]
fn fixture_stats_plain() {
    check(&stats_from_path(fixture(), StatsOptions::default()).unwrap());
}

#[test]
fn fixture_stats_gzip_sniffed_without_extension() {
    let body = std::fs::read(fixture()).unwrap();
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&body).unwrap();
    let gz = enc.finish().unwrap();
    let path = std::env::temp_dir().join(format!("genoforge-tiny-{}.bin", std::process::id()));
    std::fs::write(&path, gz).unwrap();
    let stats = stats_from_path(&path, StatsOptions::default());
    std::fs::remove_file(&path).ok();
    check(&stats.unwrap());
}

#[test]
fn parallel_and_tiny_batches_match_sequential() {
    let body = std::fs::read(fixture()).unwrap();
    // Three distinct sequences in the fixture: a sketch of 3 keeps the
    // duplicate estimate exact so `check` holds for both configurations.
    let sequential = stats_from_reader(
        Reader::new(Cursor::new(&body)),
        StatsOptions {
            threads: 1,
            batch_records: 50_000,
            sketch_size: 3,
        },
    )
    .unwrap();
    let batched = stats_from_reader(
        Reader::new(Cursor::new(&body)),
        StatsOptions {
            threads: 2,
            batch_records: 1,
            sketch_size: 3,
        },
    )
    .unwrap();
    assert_eq!(sequential, batched);
    check(&batched);
}

#[test]
fn empty_file_is_all_zero() {
    let stats = stats_from_reader(Reader::new(Cursor::new(b"")), StatsOptions::default()).unwrap();
    assert_eq!(stats.total_reads, 0);
    assert_eq!(stats.max_len, 0);
    assert_eq!(stats.per_position_mean_qual, Vec::<f64>::new());
}

#[test]
fn parse_error_surfaces_with_record_index() {
    let bad = b"@r1\nACGT\n+\nIIII\n@r2\nACGT\n+\nII\n";
    let err =
        stats_from_reader(Reader::new(Cursor::new(&bad[..])), StatsOptions::default()).unwrap_err();
    match err {
        Error::Parse { record, .. } => assert_eq!(record, 2),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn stats_round_trip_through_json() {
    let stats = stats_from_path(fixture(), StatsOptions::default()).unwrap();
    let json = serde_json::to_string(&stats).unwrap();
    let back: FastqStats = serde_json::from_str(&json).unwrap();
    assert_eq!(stats, back);
}
