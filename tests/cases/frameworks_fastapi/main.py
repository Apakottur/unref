import fastapi

import routers
from routers import router


app = fastapi.FastAPI()
app.include_router(router)
app.include_router(routers.admin)
app.include_router(routers.internal)


@app.get("/")
def root():
    return {}


@app.exception_handler(ValueError)
async def value_error(request, error):
    return None


@app.middleware("http")
async def timing(request, call_next):
    return await call_next(request)


# A router of another module.
@router.patch("/items/{item_id}")
def update_item(item_id: int):
    return item_id


@routers.admin.put("/settings")
def update_settings():
    return None


# Not a FastAPI object: a decorator with the same name is not a route.
class Registry:
    def get(self, path):
        return lambda function: function


registry = Registry()


@registry.get("/")
def not_a_route():  # expect: 5 [unused-function]
    return None


# An application made in a function is not followed.
def create_app():  # expect: 5 [unused-function]
    local = fastapi.FastAPI()

    @local.get("/")
    def local_root():  # expect: 9 [unused-function] transitive
        return {}

    return local


print(routers.Health)
