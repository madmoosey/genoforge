"""WSGI entry point (gunicorn in the container image)."""

import os

from django.core.wsgi import get_wsgi_application

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "genoforge.settings.prod")

application = get_wsgi_application()
