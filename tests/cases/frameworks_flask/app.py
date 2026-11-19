import flask

import views
from views import bp


class AdminApp(flask.Flask):
    pass


app = flask.Flask(__name__)
admin = AdminApp("admin")
app.register_blueprint(bp)
app.register_blueprint(views.api)


# Flask calls the functions that the decorators of an application register.
@app.route("/")
def index():
    return ""


@app.errorhandler(404)
def not_found(error):
    return "", 404


@app.before_request
def load_user():
    return None


@app.teardown_appcontext
def close_db(exception):
    return None


@app.template_filter("reverse")
def reverse(text):
    return text[::-1]


@app.shell_context_processor
def shell_context():
    return {}


# An instance of a subclass of `Flask`.
@admin.get("/")
def admin_index():
    return ""


# A blueprint of another module.
@bp.post("/items")
def create_item():
    return ""


@views.api.delete("/items/<int:item_id>")
def delete_item(item_id):
    return ""


# Not a Flask object: a decorator with the same name is not a route.
class Registry:
    def route(self, path):
        return lambda function: function


registry = Registry()


@registry.route("/")
def not_a_route():  # expect: 5 [unused-function]
    return None


# The Flask CLI calls a factory named `create_app` or `make_app` that makes an application, and
# Flask calls the functions that the decorators of an application or a blueprint made in a function
# register.
def create_app():
    local = flask.Flask(__name__)
    blueprint = flask.Blueprint("local", __name__)

    @local.route("/")
    def local_index():
        return ""

    @blueprint.route("/")
    def blueprint_index():
        return ""

    return local


def make_app():  # expect: 5 [unused-function]
    return registry


# Not named as a factory that the Flask CLI finds.
def build_app():  # expect: 5 [unused-function]
    return AdminApp("built")


# `current_app` stands for the application.
@flask.current_app.template_global()
def site_name():
    return ""


views.Items, views.Page
