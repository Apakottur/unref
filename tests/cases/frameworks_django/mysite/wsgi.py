import os

from django.core.wsgi import get_wsgi_application

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "mysite.settings")

# The server loads the application by a dotted path.
application = get_wsgi_application()
