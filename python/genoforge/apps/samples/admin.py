from django.contrib import admin

from .models import Sample, SequencingRun


@admin.register(Sample)
class SampleAdmin(admin.ModelAdmin):
    list_display = ("external_id", "organism", "created_at")
    search_fields = ("external_id", "organism")
    readonly_fields = ("created_at", "updated_at")


@admin.register(SequencingRun)
class SequencingRunAdmin(admin.ModelAdmin):
    list_display = ("id", "sample", "platform", "read_layout", "fastq_sha256", "created_at")
    list_filter = ("platform", "read_layout")
    list_select_related = ("sample",)
    search_fields = ("sample__external_id", "fastq_sha256", "fastq_r1")
    autocomplete_fields = ("sample",)
    readonly_fields = ("created_at", "updated_at")
