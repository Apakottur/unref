import flask

from app import AdminApp, Registry


# Flask calls the functions that the decorators of a parameter annotated as an application or a
# blueprint register.
def init_app(app: flask.Flask):
    @app.before_request
    def check_session():
        return None

    @app.cli.command()
    def seed():
        return None


def init_admin(admin: "AdminApp"):
    @admin.errorhandler(404)
    def admin_not_found(error):
        return "", 404


def init_blueprint(bp: flask.Blueprint):
    @bp.route("/")
    def blueprint_index():
        return ""


# A parameter without an annotation can be anything.
def init_other(app):
    @app.before_request
    def not_registered():  # expect: 9 [unused-function]
        return None


# Not a Flask object.
def init_registry(registry: Registry):
    @registry.route("/")
    def not_a_route():  # expect: 9 [unused-function]
        return None


print(init_app, init_admin, init_blueprint, init_other, init_registry)
