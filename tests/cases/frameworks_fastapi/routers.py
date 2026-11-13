from fastapi import APIRouter
from fastapi.routing import APIRouter as RoutingAPIRouter


class AdminRouter(APIRouter):
    pass


router = APIRouter()
admin = AdminRouter(prefix="/admin")
internal = RoutingAPIRouter()


# FastAPI calls the functions that the decorators of a router register.
@router.get("/items")
def list_items():
    return []


@router.api_route("/items", methods=["PUT"])
def replace_items():
    return []


@router.websocket("/live")
async def live(websocket):
    await websocket.accept()


# An instance of a subclass of `APIRouter`.
@admin.delete("/items/{item_id}")
def delete_item(item_id: int):
    return item_id


@internal.post("/sync")
def sync():
    return None


# A method of a route class.
class Health:
    @router.head("/health")
    def check(self):
        return None

    def unused(self):  # expect: 9 [unused-method]
        return None
