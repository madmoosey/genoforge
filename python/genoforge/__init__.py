"""genoforge: Rust-accelerated genomics QC with a Django control plane.

The compiled extension lives at ``genoforge._native`` (built by maturin from
``crates/genoforge-py``). Pure-Python code in this package wraps it; nothing
here re-implements what the Rust core already does.
"""

from genoforge._native import FastqStats, core_version, fastq_stats, gc_fraction

__all__ = ["FastqStats", "__version__", "core_version", "fastq_stats", "gc_fraction"]
__version__ = "0.1.0"
