from .base import *

# Django reads the uppercase names at the top level of a settings module.
DEBUG = True
ALLOWED_HOSTS = []
INSTALLED_APPS = ["polls.apps.PollsConfig"]
DATABASES = {"default": {"ENGINE": "django.db.backends.sqlite3", "NAME": BASE_DIR / "db.sqlite3"}}

# It ignores the other names.
helper = 1  # expect: 1 [unused-variable]


def make_secret():  # expect: 5 [unused-function]
    return ""
