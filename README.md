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
make up               # postgres + mongo
```

Native development (Rust via rustup, Python via uv):

```bash
curl https://sh.rustup.rs -sSf | sh
make setup            # uv venv + maturin build of the extension + dev tools
make test             # cargo test + pytest
make lint             # rustfmt, clippy (pedantic), ruff, mypy
```

```bash
cargo run -p genoforge-cli -- gc ACGTNN
# {"gc_fraction":0.5}
```

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
