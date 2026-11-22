import flask_restx
from flask import Flask
from flask_restx import Api, Namespace, Resource

app = Flask(__name__)
api = Api(app)
ns = Namespace("todos")
cats = api.namespace("cats")
api.add_namespace(ns)


# The `route` decorator of an API or a namespace adds the class as a resource, whose methods named
# after HTTP methods Flask-RESTX calls.
@ns.route("/<int:todo_id>")
class Todo(Resource):
    @ns.doc("get_todo")
    def get(self, todo_id):
        return {}

    def put(self, todo_id):
        return {}

    def format(self):  # expect: 9 [unused-method]
        return None


@cats.route("/")
class Cats(flask_restx.Resource):
    def get(self):
        return []


@api.route("/hello")
class Hello(Resource):
    def get(self):
        return {}


# The `errorhandler` decorator of an API or a namespace registers a function, as does the
# `documentation` decorator of an API.
@api.errorhandler(ValueError)
def handle_value_error(error):
    return {}, 400


@ns.errorhandler
def default_error(error):
    return {}, 500


@api.documentation
def custom_ui():
    return ""


# Not a namespace: a method `namespace` of something else.
class Box:
    def namespace(self, name):
        return self

    def route(self, path):
        return lambda cls: cls


box = Box()
other = box.namespace("other")


@other.route("/")
class NotRouted:  # expect: 7 [unused-class]
    pass
