# A class in a function is not the class of its name at the top level of the module.
from pydantic import BaseModel


class Item(BaseModel):
    name: str


def make():
    class Item:
        name: str  # expect: 9 [unused-attribute]

    return Item


Item, make()
