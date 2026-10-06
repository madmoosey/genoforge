# genoforge

Rust and Python bioinformatics: a small, production-shaped genomics QC pipeline.

- **Rust core** parses FASTQ / BAM / CRAM / VCF and computes QC statistics,
  exposed to Python through **PyO3 + maturin**.
- **Django** owns the control plane: models and migrations in **Postgres**,
  a resumable pipeline runner, a REST API and the admin.
- **MongoDB** stores per-variant documents with flexible annotations.
- **Bash** drives the external aligner and variant caller through a pinned
  tool image.

Runs end-to-end on a laptop against the 5.4 kb phiX174 genome. A built-in
read simulator plants known SNVs, so the end-to-end test proves the pipeline
is *correct*, not just that it ran.

> Status: scaffold. The architecture and delivery plan are in
> [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md); design decisions are in
> [docs/adr](docs/adr).

## Quickstart

No local toolchain (Docker only):

```bash
make docker-test      # builds the Rust core + wheel, runs cargo test and pytest inside the image
make up               # postgres + mongo on 127.0.0.1 (trust auth, dev only)
```

If 5432 or 27017 is already taken on your machine, set `GENOFORGE_PG_PORT` /
`GENOFORGE_MONGO_PORT` before `make up`.

Native development (Rust via rustup, Python via uv):

```bash
curl https://sh.rustup.rs -sSf | sh
make setup            # uv venv + maturin build of the extension + dev tools
make test             # cargo test + pytest
make lint             # rustfmt, clippy (pedantic), ruff, mypy
```

```bash
cargo run --release -p genoforge-cli -- fastq-stats reads.fastq.gz | jq
# {
#   "total_reads": 1000000, "total_bases": 150000000, "min_len": 150, "max_len": 150,
#   "mean_len": 150.0, "mean_qual": 35.2, "gc_frac": 0.41, "n_frac": 0.0002,
#   "q20_frac": 0.98, "q30_frac": 0.93, "dup_frac_est": 0.07,
#   "per_position_mean_qual": [36.1, 36.0, ...]
# }
make bench            # criterion: fastq_stats throughput, 1 thread vs all cores
```

The FASTQ path is streaming (one batch of records in memory at a time), sniffs
gzip from the magic bytes, parallelises across a rayon pool with one
accumulator per task and no locks, and estimates the duplicate-read fraction
from a bottom-k sketch in `O(k)` memory.

## Layout

```
crates/genoforge-core   pure Rust library: parsing, QC, simulation
crates/genoforge-cli    `genoforge` binary, JSON output
crates/genoforge-py     PyO3 bindings → genoforge._native
python/genoforge        Python package (Django project arrives in PR 4)
tests/python            unit / integration / e2e
docs/                   architecture, ADRs, schema, runbook, benchmarks
```

## License

MIT
