from flask import Blueprint
from flask.blueprints import Blueprint as RenamedBlueprint
from flask.views import MethodView, View


bp = Blueprint("items", __name__)
api = RenamedBlueprint("api", __name__)


# Flask calls the functions that the decorators of a blueprint register, also for the application.
@bp.route("/items")
def list_items():
    return ""


@bp.app_errorhandler(500)
def server_error(error):
    return "", 500


@api.before_app_request
def check_token():
    return None


# Flask calls the methods of a method view named after HTTP methods.
class Items(MethodView):
    # Flask reads the class attributes of a view that configure it.
    decorators = [print]
    init_every_request = False

    # Not an attribute of Flask's views.
    method_decorators = [print]  # expect: 5 [unused-attribute]

    def get(self):
        return ""

    def post(self):
        return ""

    def helper(self):  # expect: 9 [unused-method]
        return None


# Flask calls the `dispatch_request` method of a view.
class Page(View):
    methods = ["GET", "POST"]

    def dispatch_request(self):
        return ""

    # Not a method view: an HTTP method name is not called.
    def options(self):  # expect: 9 [unused-method]
        return ""


# Not an application: the Flask CLI does not load it.
application = dict()  # expect: 1 [unused-variable]
