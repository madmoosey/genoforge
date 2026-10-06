"""The schema's rules, exercised at both layers Django enforces them:
``full_clean()`` (ValidationError, named constraint) and the database
(IntegrityError inside a savepoint so the test transaction survives)."""

from collections.abc import Callable
from datetime import timedelta
from typing import Any

import pytest
from django.core.exceptions import ValidationError
from django.db import IntegrityError, transaction
from django.db.models import Model, ProtectedError
from django.utils import timezone

from genoforge.apps.runs.models import (
    FileArtifact,
    PipelineRun,
    PipelineStep,
    QcMetric,
    RunStatus,
    StepName,
)
from genoforge.apps.samples.models import Sample, SequencingRun

SHA_A = "a" * 64
SHA_B = "b" * 64


def rejected_by_db(create: Callable[[], Model]) -> None:
    with pytest.raises(IntegrityError), transaction.atomic():
        create()


def rejected_by_clean(instance: Model, message: str) -> None:
    """full_clean() raises with the constraint's human message."""
    with pytest.raises(ValidationError) as exc:
        instance.full_clean()
    assert message in str(exc.value), str(exc.value)


# --- samples -----------------------------------------------------------------


def test_str_representations(
    sample: Sample, seq_run: SequencingRun, pipeline_run: PipelineRun
) -> None:
    assert str(sample) == "phix-sim-0001"
    assert str(seq_run) == f"{sample.id}:SIMULATED:PE:aaaaaaaa"
    assert str(pipeline_run) == f"{pipeline_run.id} [pending]"


def test_sample_external_id_is_unique(sample: Sample) -> None:
    rejected_by_db(lambda: Sample.objects.create(external_id=sample.external_id, organism="x"))


def test_same_fastq_bytes_never_create_a_second_run(seq_run: SequencingRun) -> None:
    dup = SequencingRun(
        sample=seq_run.sample,
        platform=seq_run.platform,
        read_layout=seq_run.read_layout,
        fastq_r1="elsewhere_R1.fastq.gz",
        fastq_r2="elsewhere_R2.fastq.gz",
        fastq_sha256=seq_run.fastq_sha256,
    )
    rejected_by_clean(dup, "already exists")
    rejected_by_db(dup.save)


def test_same_fastq_bytes_under_a_different_sample_is_fine(seq_run: SequencingRun) -> None:
    other = Sample.objects.create(external_id="phix-sim-0002", organism="phiX174")
    SequencingRun.objects.create(
        sample=other,
        platform=SequencingRun.Platform.SIMULATED,
        read_layout=SequencingRun.Layout.SE,
        fastq_r1="r1.fastq.gz",
        fastq_sha256=seq_run.fastq_sha256,
    )
    assert SequencingRun.objects.filter(fastq_sha256=SHA_A).count() == 2


@pytest.mark.parametrize(
    ("layout", "r2"),
    [(SequencingRun.Layout.PE, ""), (SequencingRun.Layout.SE, "r2.fastq.gz")],
    ids=["pe-missing-r2", "se-with-r2"],
)
def test_layout_must_match_fastq_files(sample: Sample, layout: str, r2: str) -> None:
    run = SequencingRun(
        sample=sample,
        platform="ILLUMINA",
        read_layout=layout,
        fastq_r1="r1.fastq.gz",
        fastq_r2=r2,
        fastq_sha256=SHA_B,
    )
    rejected_by_clean(run, "Paired-end runs need fastq_r2")
    rejected_by_db(run.save)


def test_sha256_must_be_lowercase_hex(sample: Sample) -> None:
    run = SequencingRun(
        sample=sample,
        platform="ILLUMINA",
        read_layout="SE",
        fastq_r1="r1.fastq.gz",
        fastq_sha256="A" * 64,
    )
    with pytest.raises(ValidationError) as exc:
        run.full_clean()
    assert "fastq_sha256" in exc.value.message_dict


def test_sample_is_protected_while_runs_exist(sample: Sample, seq_run: SequencingRun) -> None:
    with pytest.raises(ProtectedError):
        sample.delete()
    seq_run.delete()
    sample.delete()
    assert not Sample.objects.exists()


# --- runs --------------------------------------------------------------------


def test_sequencing_run_is_protected_while_pipeline_runs_exist(
    seq_run: SequencingRun, pipeline_run: PipelineRun
) -> None:
    with pytest.raises(ProtectedError):
        seq_run.delete()


def test_deleting_a_run_cascades_to_everything_it_owns(pipeline_run: PipelineRun) -> None:
    PipelineStep.objects.create(run=pipeline_run, name=StepName.QC)
    FileArtifact.objects.create(
        run=pipeline_run, kind=FileArtifact.Kind.BAM, path="x.bam", bytes=10, sha256=SHA_A
    )
    QcMetric.objects.create(run=pipeline_run, stage=QcMetric.Stage.FASTQ, total_reads=4)
    pipeline_run.delete()
    assert not PipelineStep.objects.exists()
    assert not FileArtifact.objects.exists()
    assert not QcMetric.objects.exists()
    assert SequencingRun.objects.exists()  # inputs survive


def test_run_cannot_finish_before_it_starts(pipeline_run: PipelineRun) -> None:
    pipeline_run.finished_at = timezone.now()
    rejected_by_clean(pipeline_run, "cannot finish before it started")
    rejected_by_db(pipeline_run.save)


def test_duration(pipeline_run: PipelineRun) -> None:
    assert pipeline_run.duration is None
    pipeline_run.started_at = timezone.now()
    pipeline_run.finished_at = pipeline_run.started_at + timedelta(seconds=90)
    pipeline_run.save()
    pipeline_run.refresh_from_db()
    assert pipeline_run.duration == timedelta(seconds=90)


def test_step_attempts_are_unique_and_positive(pipeline_run: PipelineRun) -> None:
    PipelineStep.objects.create(run=pipeline_run, name=StepName.ALIGN, attempt=1)
    PipelineStep.objects.create(run=pipeline_run, name=StepName.ALIGN, attempt=2)
    rejected_by_db(
        lambda: PipelineStep.objects.create(run=pipeline_run, name=StepName.ALIGN, attempt=2)
    )
    rejected_by_db(
        lambda: PipelineStep.objects.create(run=pipeline_run, name=StepName.CRAM, attempt=0)
    )
    assert list(pipeline_run.steps.values_list("attempt", flat=True)) == [1, 2]


def test_artifact_path_is_unique_per_run_and_kind(pipeline_run: PipelineRun) -> None:
    make: dict[str, Any] = {"run": pipeline_run, "path": "out/x", "bytes": 1, "sha256": SHA_A}
    FileArtifact.objects.create(kind=FileArtifact.Kind.BAM, **make)
    FileArtifact.objects.create(kind=FileArtifact.Kind.BAI, **make)  # same path, other kind
    rejected_by_db(lambda: FileArtifact.objects.create(kind=FileArtifact.Kind.BAM, **make))


def test_one_metric_row_per_stage(pipeline_run: PipelineRun) -> None:
    QcMetric.objects.create(run=pipeline_run, stage=QcMetric.Stage.FASTQ)
    QcMetric.objects.create(run=pipeline_run, stage=QcMetric.Stage.BAM)
    rejected_by_db(lambda: QcMetric.objects.create(run=pipeline_run, stage=QcMetric.Stage.FASTQ))


@pytest.mark.parametrize("field", QcMetric.FRACTION_FIELDS)
def test_fractions_live_in_the_unit_interval(pipeline_run: PipelineRun, field: str) -> None:
    ok = QcMetric(run=pipeline_run, stage=QcMetric.Stage.FASTQ, **{field: 1.0})
    ok.full_clean()
    bad = QcMetric(run=pipeline_run, stage=QcMetric.Stage.BAM, **{field: 1.5})
    rejected_by_clean(bad, f"{field} must be between 0 and 1")
    rejected_by_db(bad.save)


def test_details_round_trip_and_are_queryable(pipeline_run: PipelineRun) -> None:
    per_pos = [25.0, 25.0, 33.3]
    QcMetric.objects.create(
        run=pipeline_run,
        stage=QcMetric.Stage.FASTQ,
        total_reads=4,
        details={"per_position_mean_qual": per_pos, "tool": {"name": "genoforge", "v": "0.1.0"}},
    )
    m = QcMetric.objects.get(run=pipeline_run, stage="fastq")
    assert m.details["per_position_mean_qual"] == per_pos
    assert QcMetric.objects.filter(details__tool__name="genoforge").count() == 1
    assert QcMetric.objects.filter(details__contains={"tool": {"v": "0.1.0"}}).count() == 1
    assert QcMetric.objects.filter(details__has_key="missing").count() == 0


def test_status_choices_are_shared_between_run_and_step(pipeline_run: PipelineRun) -> None:
    step = PipelineStep.objects.create(
        run=pipeline_run, name=StepName.LOAD, status=RunStatus.FAILED
    )
    pipeline_run.status = RunStatus.FAILED
    pipeline_run.save(update_fields=["status"])
    assert (
        PipelineRun.objects.filter(status=RunStatus.FAILED, steps__status=RunStatus.FAILED).get()
        == pipeline_run
    )
    assert step.get_status_display() == "Failed"
