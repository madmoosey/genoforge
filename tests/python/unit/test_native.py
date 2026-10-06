"""Contract tests: the compiled extension behaves like its stubs and like a
slow pure-Python reference."""

import math

from hypothesis import given
from hypothesis import strategies as st

import genoforge


def _reference_gc(seq: bytes) -> float:
    acgt = [b for b in seq.upper() if b in b"ACGT"]
    if not acgt:
        return 0.0
    return sum(1 for b in acgt if b in b"GC") / len(acgt)


def test_core_version_is_a_semver_string() -> None:
    parts = genoforge.core_version().split(".")
    assert len(parts) == 3
    assert all(p.isdigit() for p in parts)


def test_python_version_matches_core() -> None:
    assert genoforge.__version__ == genoforge.core_version()


def test_gc_fraction_basic() -> None:
    assert genoforge.gc_fraction(b"GGCC") == 1.0
    assert genoforge.gc_fraction(b"ATAT") == 0.0
    assert genoforge.gc_fraction(b"") == 0.0
    assert genoforge.gc_fraction(b"NNNN") == 0.0


@given(st.binary(max_size=2_000))
def test_gc_fraction_agrees_with_reference(seq: bytes) -> None:
    assert math.isclose(genoforge.gc_fraction(seq), _reference_gc(seq), abs_tol=1e-12)
