import django.core.asgi


def get_wsgi_application():
    return None


application = django.core.asgi.get_asgi_application()

# Only an application that Django makes.
other_application = get_wsgi_application()  # expect: 1 [unused-variable]
