"""Migrations are part of the contract: the committed files must match the
models, and they must apply backwards as well as forwards."""

import pytest
from django.core.management import call_command


@pytest.mark.django_db
def test_migrations_match_models() -> None:
    try:
        call_command("makemigrations", "--check", "--dry-run", verbosity=0)
    except SystemExit as exc:  # makemigrations --check exits 1 on drift
        pytest.fail(f"model changes without a migration (exit {exc.code})")


@pytest.mark.django_db(transaction=True)
def test_migrations_round_trip() -> None:
    call_command("migrate", "runs", "zero", verbosity=0)
    call_command("migrate", "samples", "zero", verbosity=0)
    call_command("migrate", verbosity=0)
