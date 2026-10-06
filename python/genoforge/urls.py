"""Root URL configuration. The REST API and health endpoints arrive in PR 9."""

from django.contrib import admin
from django.urls import path

urlpatterns = [
    path("admin/", admin.site.urls),
]
