import enum
from typing import ClassVar

import strawberry
from strawberry import federation


# Strawberry reads the fields of a type, an input and an interface, and calls their resolvers.
@strawberry.type
class User:
    name: str
    age: int
    kind: ClassVar[str] = "user"  # expect: 5 [unused-attribute]

    @strawberry.field
    def greeting(self) -> str:
        return self.name

    def helper(self) -> str:  # expect: 9 [unused-method]
        return self.name


@strawberry.input
class UserInput:
    name: str


@strawberry.interface
class Node:
    id: strawberry.ID


@federation.type(keys=["id"])
class Product:
    id: strawberry.ID


@strawberry.type
class Mutation:
    @strawberry.mutation
    def create_user(self, data: UserInput) -> User:
        return User(name=data.name, age=0)


# All the members of an enum that Strawberry registers are in the schema.
@strawberry.enum
class Role(enum.Enum):
    ADMIN = "admin"
    VIEWER = "viewer"


# Not a type of Strawberry: its fields are reported as usual.
class Plain:
    label: str  # expect: 5 [unused-attribute]

    @staticmethod
    def field(function):
        return function


class Resolvers:
    @Plain.field
    def resolve(self) -> str:  # expect: 9 [unused-method]
        return ""


schema = strawberry.Schema(query=User, mutation=Mutation, types=[Node, Product])
schema, Role.ADMIN, Plain, Resolvers
