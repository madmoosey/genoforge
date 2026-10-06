"""One execution of the pipeline over a sequencing run, its per-stage
attempts, the files it produced and the QC it measured.

Design notes (see docs/SCHEMA.md):
- ``PipelineRun`` -> ``SequencingRun`` is PROTECT: history must not vanish
  because an input row was tidied up. Everything owned by a run CASCADEs.
- Status and names are ``TextChoices`` rather than DB enums, so adding a value
  is a data migration, not an ``ALTER TYPE``.
- ``QcMetric`` keeps the metrics worth filtering on as typed columns and the
  long tail (per-position arrays, tool-specific extras) in GIN-indexed JSONB.
"""

import uuid
from datetime import timedelta

from django.contrib.postgres.indexes import GinIndex
from django.db import models
from django.db.models import Q

from genoforge.apps.common.models import sha256_validator


class RunStatus(models.TextChoices):
    PENDING = "pending", "Pending"
    RUNNING = "running", "Running"
    SUCCEEDED = "succeeded", "Succeeded"
    FAILED = "failed", "Failed"
    SKIPPED = "skipped", "Skipped"


class PipelineRun(models.Model):
    """One end-to-end execution over a ``SequencingRun``."""

    id = models.UUIDField(primary_key=True, default=uuid.uuid4, editable=False)
    seq_run = models.ForeignKey(
        "samples.SequencingRun", on_delete=models.PROTECT, related_name="pipeline_runs"
    )
    pipeline_version = models.CharField(max_length=64, help_text="git describe of the code")
    status = models.CharField(max_length=16, choices=RunStatus.choices, default=RunStatus.PENDING)
    created_at = models.DateTimeField(auto_now_add=True)
    started_at = models.DateTimeField(null=True, blank=True)
    finished_at = models.DateTimeField(null=True, blank=True)
    error = models.TextField(blank=True, default="")

    class Meta:
        ordering = ["-created_at"]
        indexes = [
            models.Index(fields=["status", "-started_at"], name="pipelinerun_status_started"),
            # The dashboard's "what is running now" query, kept tiny.
            models.Index(
                fields=["seq_run"],
                condition=Q(status="running"),
                name="pipelinerun_running_idx",
            ),
        ]
        constraints = [
            models.CheckConstraint(
                condition=Q(finished_at__isnull=True) | Q(started_at__isnull=False),
                name="pipelinerun_finish_needs_start",
                violation_error_message="A run cannot finish before it started.",
            ),
        ]

    def __str__(self) -> str:
        return f"{self.id} [{self.status}]"

    @property
    def duration(self) -> timedelta | None:
        if self.started_at is None or self.finished_at is None:
            return None
        return self.finished_at - self.started_at


class StepName(models.TextChoices):
    QC = "qc", "FASTQ QC"
    ALIGN = "align", "Align"
    CRAM = "cram", "CRAM"
    CALL = "call", "Variant call"
    LOAD = "load", "Load variants"


class PipelineStep(models.Model):
    """One attempt at one stage. Retries create new rows, so the history of
    a flaky stage is visible rather than overwritten."""

    run = models.ForeignKey(PipelineRun, on_delete=models.CASCADE, related_name="steps")
    name = models.CharField(max_length=32, choices=StepName.choices)
    attempt = models.PositiveSmallIntegerField(default=1)
    status = models.CharField(max_length=16, choices=RunStatus.choices, default=RunStatus.PENDING)
    started_at = models.DateTimeField(null=True, blank=True)
    finished_at = models.DateTimeField(null=True, blank=True)
    wall_ms = models.PositiveIntegerField(null=True, blank=True)
    max_rss_kb = models.PositiveBigIntegerField(null=True, blank=True)
    stdout_tail = models.TextField(blank=True, default="")
    stderr_tail = models.TextField(blank=True, default="")

    class Meta:
        ordering = ["run", "name", "attempt"]
        constraints = [
            models.UniqueConstraint(
                fields=["run", "name", "attempt"], name="pipelinestep_unique_attempt"
            ),
            models.CheckConstraint(
                condition=Q(attempt__gte=1),
                name="pipelinestep_attempt_positive",
            ),
        ]

    def __str__(self) -> str:
        return f"{self.name}#{self.attempt} [{self.status}]"


class FileArtifact(models.Model):
    """A file a run produced, with enough to detect it changed underneath us."""

    class Kind(models.TextChoices):
        FASTQ = "FASTQ", "FASTQ"
        BAM = "BAM", "BAM"
        BAI = "BAI", "BAM index"
        CRAM = "CRAM", "CRAM"
        CRAI = "CRAI", "CRAM index"
        VCF = "VCF", "VCF"
        TBI = "TBI", "Tabix index"

    run = models.ForeignKey(PipelineRun, on_delete=models.CASCADE, related_name="artifacts")
    kind = models.CharField(max_length=8, choices=Kind.choices)
    path = models.CharField(max_length=1024)
    bytes = models.PositiveBigIntegerField()
    sha256 = models.CharField(max_length=64, validators=[sha256_validator])

    class Meta:
        ordering = ["run", "kind", "path"]
        constraints = [
            models.UniqueConstraint(
                fields=["run", "kind", "path"], name="fileartifact_unique_path"
            ),
        ]

    def __str__(self) -> str:
        return f"{self.kind} {self.path}"


FRACTION_FIELDS = ("gc_frac", "n_frac", "q20_frac", "q30_frac", "dup_frac_est", "mapped_frac")
"""QcMetric columns that must lie in [0, 1]; each gets a named check constraint."""


def _fraction_constraint(field: str) -> models.CheckConstraint:
    return models.CheckConstraint(
        condition=Q(**{f"{field}__isnull": True}) | Q(**{f"{field}__gte": 0, f"{field}__lte": 1}),
        name=f"qcmetric_{field}_unit_interval",
        violation_error_message=f"{field} must be between 0 and 1.",
    )


class QcMetric(models.Model):
    """QC summary for one stage of one run. Typed columns for what gets
    filtered and charted; ``details`` for everything else."""

    class Stage(models.TextChoices):
        FASTQ = "fastq", "FASTQ"
        BAM = "bam", "BAM"
        VCF = "vcf", "VCF"

    run = models.ForeignKey(PipelineRun, on_delete=models.CASCADE, related_name="qc_metrics")
    stage = models.CharField(max_length=8, choices=Stage.choices)

    # fastq
    total_reads = models.BigIntegerField(null=True, blank=True)
    total_bases = models.BigIntegerField(null=True, blank=True)
    mean_len = models.FloatField(null=True, blank=True)
    mean_qual = models.FloatField(null=True, blank=True)
    gc_frac = models.FloatField(null=True, blank=True)
    n_frac = models.FloatField(null=True, blank=True)
    q20_frac = models.FloatField(null=True, blank=True)
    q30_frac = models.FloatField(null=True, blank=True)
    dup_frac_est = models.FloatField(null=True, blank=True)
    # bam
    mapped_frac = models.FloatField(null=True, blank=True)
    mean_cov = models.FloatField(null=True, blank=True)
    # vcf
    n_variants = models.IntegerField(null=True, blank=True)
    ti_tv = models.FloatField(null=True, blank=True)
    het_hom = models.FloatField(null=True, blank=True)

    details = models.JSONField(default=dict, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)

    FRACTION_FIELDS = FRACTION_FIELDS  # re-exported for callers and tests

    class Meta:
        ordering = ["run", "stage"]
        constraints = [
            models.UniqueConstraint(fields=["run", "stage"], name="qcmetric_unique_stage"),
            *(_fraction_constraint(f) for f in FRACTION_FIELDS),
        ]
        indexes = [GinIndex(fields=["details"], name="qcmetric_details_gin")]

    def __str__(self) -> str:
        return f"{self.stage} metrics for {self.run_id}"
