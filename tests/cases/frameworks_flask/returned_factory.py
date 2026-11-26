import flask


class Api(flask.Flask):
    pass


# A factory can return the application directly, an instance of a subclass of Flask or not.
def create_app():
    return flask.Flask(__name__)


def make_app():
    return Api(__name__)


app = make_app()


@app.route("/")
def index():
    return ""


# Not named as a factory that the Flask CLI finds.
def build_app():  # expect: 5 [unused-function]
    return flask.Flask(__name__)
