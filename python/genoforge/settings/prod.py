"""Production: everything sensitive must come from the environment, and the
process refuses to start without it.
"""

from .base import *  # noqa: F403
from .base import env

DEBUG = False
SECRET_KEY = env.str("GENOFORGE_SECRET_KEY")  # ImproperlyConfigured if unset
ALLOWED_HOSTS = env.list("GENOFORGE_ALLOWED_HOSTS")
CSRF_TRUSTED_ORIGINS = env.list("GENOFORGE_CSRF_TRUSTED_ORIGINS", default=[])

SECURE_PROXY_SSL_HEADER = ("HTTP_X_FORWARDED_PROTO", "https")
SECURE_SSL_REDIRECT = env.bool("GENOFORGE_SSL_REDIRECT", default=True)
SESSION_COOKIE_SECURE = True
CSRF_COOKIE_SECURE = True
SECURE_HSTS_SECONDS = 60 * 60 * 24 * 30
SECURE_HSTS_INCLUDE_SUBDOMAINS = True
