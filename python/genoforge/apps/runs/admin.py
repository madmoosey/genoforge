"""Admin as the ops console: runs with their steps, artifacts and metrics
inline; every list view select_related so paging stays O(1) in queries."""

from django.contrib import admin

from .models import FileArtifact, PipelineRun, PipelineStep, QcMetric


class ReadOnlyInline(admin.TabularInline):
    extra = 0
    can_delete = False
    show_change_link = True

    def has_add_permission(self, request, obj=None):  # type: ignore[no-untyped-def]
        return False


class PipelineStepInline(ReadOnlyInline):
    model = PipelineStep
    fields = ("name", "attempt", "status", "started_at", "finished_at", "wall_ms", "max_rss_kb")
    readonly_fields = fields


class FileArtifactInline(ReadOnlyInline):
    model = FileArtifact
    fields = ("kind", "path", "bytes", "sha256")
    readonly_fields = fields


class QcMetricInline(ReadOnlyInline):
    model = QcMetric
    fields = ("stage", "total_reads", "mean_qual", "q30_frac", "dup_frac_est", "mapped_frac")
    readonly_fields = fields


@admin.register(PipelineRun)
class PipelineRunAdmin(admin.ModelAdmin):
    list_display = ("id", "seq_run", "status", "pipeline_version", "started_at", "duration")
    list_filter = ("status", "pipeline_version")
    list_select_related = ("seq_run", "seq_run__sample")
    search_fields = ("id", "seq_run__sample__external_id")
    date_hierarchy = "created_at"
    readonly_fields = ("id", "created_at", "duration")
    autocomplete_fields = ("seq_run",)
    inlines = (PipelineStepInline, FileArtifactInline, QcMetricInline)


@admin.register(PipelineStep)
class PipelineStepAdmin(admin.ModelAdmin):
    list_display = ("run", "name", "attempt", "status", "wall_ms", "max_rss_kb", "finished_at")
    list_filter = ("name", "status")
    list_select_related = ("run",)
    search_fields = ("run__id",)
    readonly_fields = ("run",)


@admin.register(FileArtifact)
class FileArtifactAdmin(admin.ModelAdmin):
    list_display = ("run", "kind", "path", "bytes")
    list_filter = ("kind",)
    list_select_related = ("run",)
    search_fields = ("path", "sha256")


@admin.register(QcMetric)
class QcMetricAdmin(admin.ModelAdmin):
    list_display = ("run", "stage", "total_reads", "mean_qual", "q30_frac", "mapped_frac")
    list_filter = ("stage",)
    list_select_related = ("run",)
    readonly_fields = ("created_at",)
