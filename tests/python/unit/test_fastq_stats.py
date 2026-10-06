"""Contract tests for ``genoforge.fastq_stats``.

The fixture reads mirror ``crates/genoforge-core/tests/fixtures/tiny.fastq``
and the expected values are worked out by hand in the same way:
r1/r3 = 10 bases Q40 (5 GC), r2 = 10 bases Q20 (8 GC, 2 N), r4 = 2 bases Q0.
"""

import gzip
import json
import math
import tempfile
from collections.abc import Iterator, Sequence
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

from genoforge import FastqStats, fastq_stats

TINY = (
    "@r1 first read\nACGTACGTAC\n+\nIIIIIIIIII\n"
    "@r2\nGGGGCCCCNN\n+r2\n5555555555\n"
    "@r3 duplicate of r1\nACGTACGTAC\n+\nIIIIIIIIII\n"
    "@r4\nAT\n+\n!!\n"
)

EXPECTED = {
    "total_reads": 4,
    "total_bases": 32,
    "min_len": 2,
    "max_len": 10,
    "mean_len": 8.0,
    "mean_qual": 1000 / 32,
    "gc_frac": 18 / 30,
    "n_frac": 2 / 32,
    "q20_frac": 30 / 32,
    "q30_frac": 20 / 32,
    "dup_frac_est": 0.25,
    "per_position_mean_qual": [25.0, 25.0] + [100 / 3] * 8,
}


def _assert_matches(stats: FastqStats, expected: dict[str, object]) -> None:
    for key, want in expected.items():
        got = getattr(stats, key)
        if isinstance(want, list):
            assert len(got) == len(want), key
            for g, w in zip(got, want, strict=True):
                assert math.isclose(g, w, abs_tol=1e-9), key
        elif isinstance(want, float):
            assert math.isclose(got, want, abs_tol=1e-9), key
        else:
            assert got == want, key


@pytest.fixture
def tiny_fastq(tmp_path: Path) -> Path:
    p = tmp_path / "tiny.fastq"
    p.write_text(TINY)
    return p


@pytest.fixture
def tiny_fastq_gz_no_extension(tmp_path: Path) -> Path:
    p = tmp_path / "tiny.bin"
    p.write_bytes(gzip.compress(TINY.encode()))
    return p


def test_fixture_values(tiny_fastq: Path) -> None:
    _assert_matches(fastq_stats(tiny_fastq), EXPECTED)


def test_accepts_str_and_pathlike_and_gzip(
    tiny_fastq: Path, tiny_fastq_gz_no_extension: Path
) -> None:
    _assert_matches(fastq_stats(str(tiny_fastq)), EXPECTED)
    _assert_matches(fastq_stats(tiny_fastq_gz_no_extension), EXPECTED)


def test_threads_and_batches_do_not_change_results(tiny_fastq: Path) -> None:
    a = fastq_stats(tiny_fastq, threads=1, batch_records=1, sketch_size=3)
    b = fastq_stats(tiny_fastq, threads=2, batch_records=50_000, sketch_size=3)
    assert a.to_dict() == b.to_dict()


def test_to_dict_round_trips_through_json(tiny_fastq: Path) -> None:
    d = fastq_stats(tiny_fastq).to_dict()
    assert set(d) == set(EXPECTED)
    assert json.loads(json.dumps(d)) == d


def test_repr_is_informative(tiny_fastq: Path) -> None:
    r = repr(fastq_stats(tiny_fastq))
    assert r.startswith("FastqStats(total_reads=4, total_bases=32")


def test_stats_are_immutable(tiny_fastq: Path) -> None:
    s = fastq_stats(tiny_fastq)
    with pytest.raises(AttributeError):
        s.total_reads = 1  # type: ignore[misc]


def test_missing_file_is_file_not_found(tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError):
        fastq_stats(tmp_path / "nope.fastq")


def test_malformed_input_is_value_error_with_record_index(tmp_path: Path) -> None:
    p = tmp_path / "bad.fastq"
    p.write_text("@r1\nACGT\n+\nIIII\n@r2\nACGT\n+\nII\n")
    with pytest.raises(ValueError, match=r"FASTQ parse error at record 2"):
        fastq_stats(p)


def test_empty_file(tmp_path: Path) -> None:
    p = tmp_path / "empty.fastq"
    p.write_bytes(b"")
    s = fastq_stats(p)
    assert s.total_reads == 0
    assert s.per_position_mean_qual == []


# --- property test against a pure-Python reference ---------------------------


def _reference(records: Sequence[tuple[bytes, bytes]]) -> dict[str, object]:
    bases = sum(len(s) for s, _ in records)
    acgt = sum(1 for s, _ in records for b in s.upper() if b in b"ACGT")
    gc = sum(1 for s, _ in records for b in s.upper() if b in b"GC")
    n = sum(1 for s, _ in records for b in s.upper() if b == ord("N"))
    phreds = [q - 33 for _, qual in records for q in qual]
    max_len = max((len(s) for s, _ in records), default=0)
    per_pos = []
    for i in range(max_len):
        col = [q[i] - 33 for _, q in records if len(q) > i]
        per_pos.append(sum(col) / len(col))
    distinct = len({s for s, _ in records})

    def frac(a: float, b: float) -> float:
        return a / b if b else 0.0

    return {
        "total_reads": len(records),
        "total_bases": bases,
        "min_len": min((len(s) for s, _ in records), default=0),
        "max_len": max_len,
        "mean_len": frac(bases, len(records)),
        "mean_qual": frac(sum(phreds), bases),
        "gc_frac": frac(gc, acgt),
        "n_frac": frac(n, bases),
        "q20_frac": frac(sum(1 for p in phreds if p >= 20), bases),
        "q30_frac": frac(sum(1 for p in phreds if p >= 30), bases),
        # exact when the sketch holds every distinct sequence
        "dup_frac_est": 1 - frac(distinct, len(records)) if records else 0.0,
        "per_position_mean_qual": per_pos,
    }


def _to_fastq(records: Sequence[tuple[bytes, bytes]]) -> bytes:
    out = bytearray()
    for i, (seq, qual) in enumerate(records):
        out += b"@r%d\n%s\n+\n%s\n" % (i, seq, qual)
    return bytes(out)


@st.composite
def _record(draw: st.DrawFn) -> tuple[bytes, bytes]:
    n = draw(st.integers(0, 80))
    seq = bytes(draw(st.lists(st.sampled_from(b"ACGTNacgtn"), min_size=n, max_size=n)))
    qual = bytes(draw(st.lists(st.integers(33, 126), min_size=n, max_size=n)))
    return seq, qual


def _tmp_fastq(data: bytes) -> Iterator[Path]:
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "reads.fastq"
        p.write_bytes(data)
        yield p


@settings(max_examples=60, deadline=None)
@given(st.lists(_record(), max_size=30))
def test_matches_pure_python_reference(records: list[tuple[bytes, bytes]]) -> None:
    for path in _tmp_fastq(_to_fastq(records)):
        stats = fastq_stats(path, sketch_size=10_000)
        _assert_matches(stats, _reference(records))
