import fastapi
from fastapi import FastAPI

from routers import AdminRouter


# An application or router made in a function, whose decorators are in the same function.
def test_local_app():
    app = FastAPI()

    @app.get("/")
    def root():
        return {}

    @app.post("/items")
    async def create_item():
        return {}

    return app


def test_local_router():
    router: fastapi.APIRouter = fastapi.APIRouter()
    admin = AdminRouter()

    @router.get("/")
    def list_items():
        return []

    @admin.delete("/")
    def delete_items():
        return None

    # A class in the function sees its local variables.
    class Health:
        @router.head("/health")
        def check(self):
            return None

    return router, admin, Health


# A local variable of the same name that is not an application, in another function.
class Registry:
    def get(self, path):
        return lambda function: function


def test_other_local():
    app = Registry()

    @app.get("/")
    def not_a_route():  # expect: 9 [unused-function]
        return None

    return app


# A function nested in the function is not followed.
def test_nested():
    app = FastAPI()

    def register():
        @app.get("/")
        def nested_route():  # expect: 13 [unused-function]
            return None

    register()
    return app


# A variable declared `global` is not local.
def test_global():
    global shared
    shared = FastAPI()

    @shared.get("/")
    def global_route():  # expect: 9 [unused-function]
        return None


# An unused function that makes an application is reported, but not its routes.
def create_app():  # expect: 5 [unused-function]
    app = FastAPI()

    @app.get("/")
    def local_root():
        return {}

    return app
