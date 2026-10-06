# ADR 0001: Rust for parsing and statistics, Python for orchestration

**Status:** accepted · **Date:** 2026-10-06

## Context

Genomic files are large and simple: FASTQ, BAM/CRAM and VCF are byte streams
of millions of small records. The expensive work is decoding bytes, counting,
hashing and binning. The interesting work around it (what to run, when, where
the results go, who can see them) is glue that changes often and benefits from
a large ecosystem.

## Decision

- Anything that touches every byte of a file is written in Rust, in
  `crates/genoforge-core`, with no Python dependency. It exposes plain
  `Result`-returning functions and iterators.
- Orchestration, persistence, the HTTP API and operator tooling are Python
  (Django). Python never re-implements a parser that the core already has.
- The boundary is one crate, `crates/genoforge-py`, built by maturin into
  `genoforge._native`. Errors are converted to Python exceptions there and
  nowhere else.

## Consequences

- Two toolchains to install. Mitigated by `make setup` (uv + maturin) and a
  Dockerfile whose `test` stage needs no local toolchain at all.
- Pure-Python contract tests must exist for every binding so a change in Rust
  cannot silently change Python behaviour.
- Benchmarks comparing the Rust path to a pure-Python reference are part of the
  repo, so the cost of the second toolchain is justified with numbers.
