import flask

from app import create_app


# The Flask CLI and WSGI servers load a variable named `app` or `application` that is set to an
# application, made by a factory or not.
app = create_app()
application = flask.Flask(__name__)
other = flask.Flask(__name__)  # expect: 1 [unused-variable]
server = create_app()


# A variable set to what a factory makes is an application.
@server.shell_context_processor
def shell_context():
    return {}
