"""What was sequenced: biological samples and the FASTQ runs produced from them.

These are long-lived entities with identity and joins, which is why they live
in Postgres rather than the document store (ADR 0002).
"""

from django.db import models
from django.db.models import Q

from genoforge.apps.common.models import TimeStamped, sha256_validator


class Sample(TimeStamped):
    """A biological sample identified by an external accession."""

    external_id = models.CharField(max_length=64, unique=True)
    organism = models.CharField(max_length=128)

    class Meta:
        ordering = ["external_id"]

    def __str__(self) -> str:
        return self.external_id


class SequencingRun(TimeStamped):
    """One FASTQ (or pair) produced from a sample. ``fastq_sha256`` is the
    ingestion idempotency key: the same bytes never create a second run."""

    class Platform(models.TextChoices):
        ILLUMINA = "ILLUMINA", "Illumina"
        ONT = "ONT", "Oxford Nanopore"
        PACBIO = "PACBIO", "PacBio"
        SIMULATED = "SIMULATED", "Simulated"

    class Layout(models.TextChoices):
        SE = "SE", "Single-end"
        PE = "PE", "Paired-end"

    sample = models.ForeignKey(Sample, on_delete=models.PROTECT, related_name="sequencing_runs")
    platform = models.CharField(max_length=16, choices=Platform.choices)
    read_layout = models.CharField(max_length=2, choices=Layout.choices)
    fastq_r1 = models.CharField(max_length=1024)
    fastq_r2 = models.CharField(max_length=1024, blank=True, default="")
    fastq_sha256 = models.CharField(max_length=64, validators=[sha256_validator])

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            models.UniqueConstraint(
                fields=["sample", "fastq_sha256"],
                name="sequencingrun_unique_sample_fastq",
            ),
            # PE needs a second file, SE must not have one.
            models.CheckConstraint(
                condition=(Q(read_layout="PE") & ~Q(fastq_r2=""))
                | (Q(read_layout="SE") & Q(fastq_r2="")),
                name="sequencingrun_layout_matches_files",
                violation_error_message=(
                    "Paired-end runs need fastq_r2; single-end runs must not have one."
                ),
            ),
        ]

    def __str__(self) -> str:
        return f"{self.sample_id}:{self.platform}:{self.read_layout}:{self.fastq_sha256[:8]}"

    @property
    def is_paired(self) -> bool:
        return self.read_layout == self.Layout.PE
