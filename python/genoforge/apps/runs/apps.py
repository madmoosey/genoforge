from django.apps import AppConfig


class RunsConfig(AppConfig):
    default_auto_field = "django.db.models.BigAutoField"
    name = "genoforge.apps.runs"
    label = "runs"
    verbose_name = "Pipeline runs"
