"""Abstract building blocks shared by the concrete apps. Not an installed app:
abstract models need no migrations."""

from django.core.validators import RegexValidator
from django.db import models

sha256_validator = RegexValidator(r"^[0-9a-f]{64}$", "Enter a lowercase hex SHA-256.")


class TimeStamped(models.Model):
    """``created_at`` / ``updated_at`` maintained by the ORM."""

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    class Meta:
        abstract = True
