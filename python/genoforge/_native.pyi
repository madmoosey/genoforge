"""Type stubs for the compiled ``genoforge._native`` extension.

Keep in sync with ``crates/genoforge-py/src/lib.rs``.
"""

def core_version() -> str:
    """Version of the compiled Rust core."""

def gc_fraction(sequence: bytes) -> float:
    """GC fraction of a nucleotide sequence, ignoring N and non-ACGT bytes.

    Borrows the bytes buffer without copying and releases the GIL while computing.
    """
