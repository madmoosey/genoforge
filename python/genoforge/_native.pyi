"""Type stubs for the compiled ``genoforge._native`` extension.

Keep in sync with ``crates/genoforge-py/src/lib.rs``.
"""

from os import PathLike
from typing import Any, Final, final

def core_version() -> str:
    """Version of the compiled Rust core."""

def gc_fraction(sequence: bytes) -> float:
    """GC fraction of a nucleotide sequence, ignoring N and non-ACGT bytes.

    Borrows the bytes buffer without copying and releases the GIL while computing.
    """

@final
class FastqStats:
    """QC summary for one FASTQ file. Immutable (attributes are read-only)."""

    total_reads: Final[int]
    total_bases: Final[int]
    min_len: Final[int]
    max_len: Final[int]
    mean_len: Final[float]
    mean_qual: Final[float]
    gc_frac: Final[float]
    n_frac: Final[float]
    q20_frac: Final[float]
    q30_frac: Final[float]
    dup_frac_est: Final[float]
    per_position_mean_qual: Final[list[float]]

    def to_dict(self) -> dict[str, Any]:
        """Plain-dict form, e.g. for JSON serialisation or a Django JSONField."""

def fastq_stats(
    path: str | PathLike[str],
    *,
    threads: int = 0,
    batch_records: int = 50_000,
    sketch_size: int = 10_000,
) -> FastqStats:
    """Streaming QC statistics for a FASTQ file (plain or gzip, sniffed by content).

    Releases the GIL for the whole computation. ``threads=0`` uses one worker per
    core. Raises ``FileNotFoundError`` / ``PermissionError`` / ``OSError`` for I/O
    problems and ``ValueError`` (with the 1-based record index) for malformed input.
    """
