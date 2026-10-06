"""Shared fixtures. Every fixture here requests ``db``, so tests that use
them run inside pytest-django's per-test transaction against Postgres."""

import pytest

from genoforge.apps.runs.models import PipelineRun
from genoforge.apps.samples.models import Sample, SequencingRun

SHA_A = "a" * 64
SHA_B = "b" * 64


@pytest.fixture
def sample(db: None) -> Sample:
    return Sample.objects.create(external_id="phix-sim-0001", organism="Escherichia phage phiX174")


@pytest.fixture
def seq_run(sample: Sample) -> SequencingRun:
    return SequencingRun.objects.create(
        sample=sample,
        platform=SequencingRun.Platform.SIMULATED,
        read_layout=SequencingRun.Layout.PE,
        fastq_r1="data/work/phix_R1.fastq.gz",
        fastq_r2="data/work/phix_R2.fastq.gz",
        fastq_sha256=SHA_A,
    )


@pytest.fixture
def pipeline_run(seq_run: SequencingRun) -> PipelineRun:
    return PipelineRun.objects.create(seq_run=seq_run, pipeline_version="0.1.0")
