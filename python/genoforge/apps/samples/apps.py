from django.apps import AppConfig


class SamplesConfig(AppConfig):
    default_auto_field = "django.db.models.BigAutoField"
    name = "genoforge.apps.samples"
    label = "samples"
    verbose_name = "Samples & sequencing runs"
