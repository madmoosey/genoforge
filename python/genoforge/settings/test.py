"""Test runs: fast password hashing, debug off so templates behave like prod."""

from .base import *  # noqa: F403

DEBUG = False
SECRET_KEY = "django-insecure-genoforge-test"  # noqa: S105 - test only
PASSWORD_HASHERS = ["django.contrib.auth.hashers.MD5PasswordHasher"]
