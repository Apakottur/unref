from flask import Flask
from flask_restful import Api, Resource

app = Flask(__name__)
api = Api(app)


# Flask-RESTful calls the methods of a resource named after HTTP methods.
class Item(Resource):
    # Flask-RESTful reads the decorators of the methods, and Flask the class attributes of a view.
    method_decorators = [print]
    decorators = [print]

    # Not read by either.
    cache = None  # expect: 5 [unused-attribute]

    def get(self, item_id):
        return {}

    def post(self, item_id):
        return {}

    def helper(self):  # expect: 9 [unused-method]
        return None


api.add_resource(Item, "/items/<item_id>")


# A subclass of a resource of the project.
class Admin(Item):
    def delete(self, item_id):
        return {}


api.add_resource(Admin, "/admin/<item_id>")


# The `resource` decorator of an API adds the class as a resource.
@api.resource("/health")
class Health(Resource):
    def get(self):
        return {}


# The `representation` decorator of an API registers a function that makes responses.
@api.representation("application/xml")
def output_xml(data, code, headers=None):
    return data


# Not a resource: a method named after an HTTP method is not called.
class Plain:  # expect: 7 [unused-class]
    def get(self):  # expect: 9 [unused-method] transitive
        return {}


# Not an API: a decorator with the same name does not add a class.
class Registry:
    def resource(self, path):
        return lambda cls: cls


registry = Registry()


@registry.resource("/other")
class NotAResource:  # expect: 7 [unused-class]
    pass


# A factory with a local API and one passed to a function.
def create_app():
    local = Api(Flask(__name__))

    @local.resource("/local")
    class Local(Resource):
        def get(self):
            return {}

    return local


def init_api(other: Api):
    @other.representation("text/csv")
    def output_csv(data, code, headers=None):
        return data


create_app, init_api
