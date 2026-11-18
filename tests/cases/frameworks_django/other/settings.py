# Not a settings module, as no `DJANGO_SETTINGS_MODULE` names it.
DEBUG = True  # expect: 1 [unused-variable]
ROOT_URLCONF = "other.urls"  # expect: 1 [unused-variable]
