import flask
import pytest

from app import create_app


# A parameter named as a fixture that returns or yields an application, or what a factory makes, is
# an application.
@pytest.fixture
def app():
    return flask.Flask(__name__)


@pytest.fixture
def factory_app():
    application = create_app()
    yield application


@pytest.fixture
def other():
    return object()


def test_index(app):
    @app.route("/")
    def index():
        return "ok"


def test_factory(factory_app):
    @factory_app.route("/")
    def index():
        return "ok"


def test_other(other):
    @other.route("/")
    def index():  # expect: 9 [unused-function]
        return "ok"
