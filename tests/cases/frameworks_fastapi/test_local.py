import importlib

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


# Starlette's decorators, which FastAPI's applications and routers have, and the webhooks of an
# application.
def test_starlette_and_webhooks():
    app = FastAPI()

    @app.route("/plain")
    def plain(request):
        return None

    @app.websocket_route("/live")
    async def live(websocket):
        return None

    @app.webhooks.post("new-subscription")
    def new_subscription(body):
        return None

    return app


# A router that is one of several values, or another variable.
def test_chosen_router(nested):
    app = FastAPI()
    router = fastapi.APIRouter() if nested else app.router
    reader = app if nested else router
    other = Registry() if nested else None

    @router.get("/items")
    def chosen():
        return []

    @reader.middleware("http")
    async def aliased(request, call_next):
        return await call_next(request)

    @other.get("/")
    def not_chosen():  # expect: 9 [unused-function]
        return None

    return app


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


# The application of some module whose name matches a pattern.
def test_tutorial(name):
    module = importlib.import_module(f"tutorials.{name}")

    @module.app.get("/check")
    def check():
        return None

    other = importlib.import_module(f"others.{name}")

    @other.app.get("/")
    def not_a_route_either():  # expect: 9 [unused-function]
        return None

    return module, other
