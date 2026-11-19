# A class or a function in a function does not hide a name that the module imports.
from pydantic import BaseModel as Base
from pydantic import BaseModel as Model


def make():
    class Base:
        pass

    def Model():
        pass

    return Base, Model


class User(Base):
    name: str


class Group(Model):
    name: str


User, Group, make()
