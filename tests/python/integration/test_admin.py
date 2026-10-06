"""The admin is the ops console, so it must render and must not do one query
per row."""

from collections.abc import Callable
from contextlib import AbstractContextManager
from typing import Any

import pytest
from django.test import Client
from django.urls import reverse

from genoforge.apps.runs.models import FileArtifact, PipelineRun, PipelineStep, QcMetric, StepName
from genoforge.apps.samples.models import Sample, SequencingRun

CHANGELISTS = [
    "admin:samples_sample_changelist",
    "admin:samples_sequencingrun_changelist",
    "admin:runs_pipelinerun_changelist",
    "admin:runs_pipelinestep_changelist",
    "admin:runs_fileartifact_changelist",
    "admin:runs_qcmetric_changelist",
]


@pytest.fixture
def populated(sample: Sample) -> list[PipelineRun]:
    runs = []
    for i in range(5):
        seq = SequencingRun.objects.create(
            sample=sample,
            platform="SIMULATED",
            read_layout="SE",
            fastq_r1=f"r{i}.fastq.gz",
            fastq_sha256=f"{i:064x}",
        )
        for _ in range(4):
            run = PipelineRun.objects.create(seq_run=seq, pipeline_version="0.1.0")
            PipelineStep.objects.create(run=run, name=StepName.QC)
            FileArtifact.objects.create(run=run, kind="BAM", path="x.bam", bytes=1, sha256="0" * 64)
            QcMetric.objects.create(run=run, stage="fastq", total_reads=1)
            runs.append(run)
    return runs


@pytest.mark.parametrize("name", CHANGELISTS)
def test_changelists_render(admin_client: Client, populated: list[PipelineRun], name: str) -> None:
    assert admin_client.get(reverse(name)).status_code == 200


def test_run_change_page_renders_with_inlines(
    admin_client: Client, populated: list[PipelineRun]
) -> None:
    url = reverse("admin:runs_pipelinerun_change", args=[populated[0].pk])
    resp = admin_client.get(url)
    assert resp.status_code == 200
    assert b"FASTQ QC" in resp.content


@pytest.mark.parametrize("name", CHANGELISTS)
def test_changelists_have_no_n_plus_one(
    admin_client: Client,
    populated: list[PipelineRun],
    django_assert_max_num_queries: Callable[[int], AbstractContextManager[Any]],
    name: str,
) -> None:
    # 20 rows with related objects; a per-row query would blow well past this.
    with django_assert_max_num_queries(12):
        admin_client.get(reverse(name))
