# Architecture

genoforge is one end-to-end genomics workflow, built so each layer is the right
tool for its job and the seams between them are explicit.

```
FASTQ (simulated or real)
   │  genoforge fastq-stats          ← Rust: streaming QC (rayon-parallel)
   ▼
minimap2 → samtools sort/index      ← Bash: scripts/align.sh (pinned tool image)
   │  genoforge bam-stats            ← Rust (noodles): mapped %, MAPQ, insert size, coverage
   ▼
samtools view -C  →  CRAM           ← reference-based compression, round-trip verified
   │
bcftools mpileup | call  →  VCF     ← Bash: scripts/call.sh
   │  genoforge vcf-stats + load     ← Rust iterator → Python → MongoDB documents
   ▼
Postgres (Django ORM): sample / run / step / qc_metric / artifact
MongoDB (pymongo):     variants
   ▼
Django REST Framework API  ·  Django admin  ·  /metrics
```

## Layers

| layer | lives in | responsibility | must not |
|---|---|---|---|
| core | `crates/genoforge-core` | parse, count, hash, simulate; `Result` everywhere | know about Python, databases or files it did not open |
| CLI | `crates/genoforge-cli` | JSON front-end to the core for Bash and benchmarks | contain logic the core lacks |
| bindings | `crates/genoforge-py` → `genoforge._native` | map core types/errors to Python; release the GIL | add behaviour |
| control plane | `python/genoforge` (Django) | models, migrations, DAG runner, management commands, API, admin | parse genomic formats in Python |
| tools | `scripts/*.sh` + pinned image | aligner, sorter, caller | be called from anywhere but the runner's `tools.py` |

## Decisions

See `docs/adr/`. The three that shape everything:

1. [Rust for parsing, Python for orchestration](adr/0001-rust-for-parsing-python-for-orchestration.md)
2. [Postgres for entities, MongoDB for variant records](adr/0002-postgres-for-entities-mongodb-for-variant-records.md)
3. [PyO3 + maturin for the boundary](adr/0003-pyo3-maturin-over-cffi-or-subprocess.md)

## Delivery

The repo is built in ten pull requests, each green in CI. The PR list is the
changelog; the PR descriptions are the design notes.

| # | PR |
|---|---|
| 1 | workspace, maturin packaging, Docker, CI, pre-commit, ADRs (this PR) |
| 2 | `fastq` module + CLI `fastq-stats` + criterion bench + proptest |
| 3 | real PyO3 bindings, `.pyi` stubs, Python contract tests |
| 4 | Django project, models + migrations + admin, pytest-django |
| 5 | `simulate` + fetch/align/cram/call scripts |
| 6 | `bam`/`vcf` modules, `bam-stats`/`vcf-stats` |
| 7 | Mongo repository, `load` stage, aggregation endpoints |
| 8 | DAG runner (retries, resume, idempotency) + end-to-end test |
| 9 | DRF API, health/ready, Prometheus, runbook, gunicorn image |
| 10 | benchmarks, `BENCHMARKS.md`, README polish |
