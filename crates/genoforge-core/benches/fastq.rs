//! Throughput of the FASTQ driver on a synthetic in-memory file.
//!
//! Run with `cargo bench -p genoforge-core`; results land in `target/criterion`.

#![allow(clippy::cast_possible_truncation, missing_docs)]

use std::hint::black_box;
use std::io::Cursor;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use genoforge_core::fastq::{stats_from_reader, Reader, StatsOptions};

const READS: usize = 200_000;
const READ_LEN: usize = 150;

/// Deterministic xorshift64* so every run benchmarks the same bytes.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}

fn synthetic_fastq(reads: usize, len: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng(seed);
    let mut out = Vec::with_capacity(reads * (len * 2 + 16));
    for i in 0..reads {
        out.extend_from_slice(format!("@read{i}\n").as_bytes());
        for _ in 0..len {
            out.push(b"ACGT"[(rng.next() % 4) as usize]);
        }
        out.extend_from_slice(b"\n+\n");
        for _ in 0..len {
            out.push(b'!' + 2 + (rng.next() % 39) as u8); // Phred 2..=40
        }
        out.push(b'\n');
    }
    out
}

fn bench_fastq_stats(c: &mut Criterion) {
    let data = synthetic_fastq(READS, READ_LEN, 0x5eed);
    let mut group = c.benchmark_group("fastq_stats");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.sample_size(10);
    for threads in [1usize, 0] {
        let label = if threads == 0 {
            "all-cores".to_string()
        } else {
            threads.to_string()
        };
        group.bench_with_input(
            BenchmarkId::new("threads", label),
            &threads,
            |b, &threads| {
                b.iter(|| {
                    stats_from_reader(
                        Reader::new(Cursor::new(black_box(&data))),
                        StatsOptions {
                            threads,
                            ..StatsOptions::default()
                        },
                    )
                    .unwrap()
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_fastq_stats);
criterion_main!(benches);
