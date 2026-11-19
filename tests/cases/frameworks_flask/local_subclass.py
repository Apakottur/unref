# An application of a subclass of Flask that is defined in the function registers its views.
import flask


def make():
    class Custom(flask.Flask):
        pass

    app = Custom(__name__)

    @app.route("/")
    def index():
        return ""

    return app


make()
