"""Settings shared by every environment. Values come from ``GENOFORGE_*`` and
``DATABASE_URL`` environment variables via django-environ; only ``dev`` and
``test`` supply insecure defaults.
"""

from pathlib import Path

import environ

# python/genoforge/settings/base.py -> repository root
BASE_DIR = Path(__file__).resolve().parents[3]

env = environ.Env()

SECRET_KEY = env.str("GENOFORGE_SECRET_KEY", default="django-insecure-genoforge-dev")
DEBUG = env.bool("GENOFORGE_DEBUG", default=False)
ALLOWED_HOSTS = env.list("GENOFORGE_ALLOWED_HOSTS", default=["localhost", "127.0.0.1"])

INSTALLED_APPS = [
    "django.contrib.admin",
    "django.contrib.auth",
    "django.contrib.contenttypes",
    "django.contrib.sessions",
    "django.contrib.messages",
    "django.contrib.staticfiles",
    "django.contrib.postgres",
    "genoforge.apps.samples",
    "genoforge.apps.runs",
]

MIDDLEWARE = [
    "django.middleware.security.SecurityMiddleware",
    "django.contrib.sessions.middleware.SessionMiddleware",
    "django.middleware.common.CommonMiddleware",
    "django.middleware.csrf.CsrfViewMiddleware",
    "django.contrib.auth.middleware.AuthenticationMiddleware",
    "django.contrib.messages.middleware.MessageMiddleware",
    "django.middleware.clickjacking.XFrameOptionsMiddleware",
]

ROOT_URLCONF = "genoforge.urls"
WSGI_APPLICATION = "genoforge.wsgi.application"

TEMPLATES = [
    {
        "BACKEND": "django.template.backends.django.DjangoTemplates",
        "DIRS": [],
        "APP_DIRS": True,
        "OPTIONS": {
            "context_processors": [
                "django.template.context_processors.request",
                "django.contrib.auth.context_processors.auth",
                "django.contrib.messages.context_processors.messages",
            ],
        },
    },
]

# Postgres only: the schema uses JSONB, GIN and partial indexes. Local default
# matches docker-compose (trust auth on loopback).
DATABASES = {
    "default": env.db_url(
        "DATABASE_URL",
        default="postgres://genoforge@127.0.0.1:5432/genoforge",
    ),
}
DATABASES["default"]["CONN_MAX_AGE"] = env.int("GENOFORGE_CONN_MAX_AGE", default=60)
DATABASES["default"]["CONN_HEALTH_CHECKS"] = True

DEFAULT_AUTO_FIELD = "django.db.models.BigAutoField"

AUTH_PASSWORD_VALIDATORS = [
    {"NAME": "django.contrib.auth.password_validation.UserAttributeSimilarityValidator"},
    {"NAME": "django.contrib.auth.password_validation.MinimumLengthValidator"},
    {"NAME": "django.contrib.auth.password_validation.CommonPasswordValidator"},
    {"NAME": "django.contrib.auth.password_validation.NumericPasswordValidator"},
]

LANGUAGE_CODE = "en-us"
TIME_ZONE = "UTC"
USE_I18N = True
USE_TZ = True

STATIC_URL = "static/"
STATIC_ROOT = BASE_DIR / "staticfiles"
