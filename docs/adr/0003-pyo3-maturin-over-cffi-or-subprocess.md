# ADR 0003: PyO3 + maturin for the Rust/Python boundary

**Status:** accepted · **Date:** 2026-10-06

## Context

Python needs to call the Rust core. Options: shell out to the `genoforge` CLI
and parse JSON; expose a C ABI and use `ctypes`/`cffi`; or build a native
extension module with PyO3.

## Decision

PyO3 with the `abi3-py312` feature, built by maturin as the `pyproject.toml`
build backend so `pip install -e .` and `uv pip install` just work.

## Rationale

- **Streaming across the boundary.** A PyO3 `#[pyclass]` iterator can yield
  VCF records lazily into Python; a subprocess would force materialising JSON.
- **Releasing the GIL.** `py.allow_threads` lets a Django request thread or a
  Celery worker keep serving while Rust parses a file.
- **Typed errors.** Core `Error` variants map to `IOError`/`ValueError` in one
  place, rather than parsing stderr.
- **abi3.** One wheel per platform covers every CPython ≥ 3.12, which keeps CI
  and release matrices small.
- **cffi** would need a hand-maintained C header and manual memory ownership
  rules for every struct; PyO3 derives that.

## Consequences

- Type stubs (`python/genoforge/_native.pyi`) must be maintained by hand and
  are checked by mypy in CI.
- The CLI remains for Bash pipelines and benchmarking; both front-ends call the
  same core so behaviour cannot drift.
