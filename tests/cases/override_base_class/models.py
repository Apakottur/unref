import collections.abc
import typing
from typing import NamedTuple

import pydantic
import pydantic as renamed_module
from pydantic import BaseModel
from pydantic import BaseModel as Renamed


class Direct(BaseModel):
    table_name = {}
    field: int  # expect: 5 [unused-attribute]


class ThroughModule(pydantic.BaseModel):
    table_name = {}


class ThroughRenamedImport(Renamed):
    table_name = {}


class ThroughRenamedModule(renamed_module.BaseModel):
    table_name = {}


# A subclass of a class of the project inherits from its base classes.
class Child(Direct):
    table_name = {}


class GrandChild(Child):
    table_name = {}


# The type arguments of a base class are ignored.
class Generic(Direct[int], typing.Generic[int]):
    table_name = {}


# A nested class is a class like any other.
class Outer:
    class Inner(BaseModel):
        table_name = {}


class ThroughNested(Outer.Inner):
    table_name = {}


# The override applies to everything in the class: attributes, methods and decorated methods.
class Row(NamedTuple):
    first: int
    second: int

    def method(self):
        return self


class Table(collections.abc.Mapping):
    @register
    def decorated(self):
        return self

    def __init__(self):
        self.on_instance = 1


# Not a base class of the override.
class Plain:
    table_name = {}  # expect: 5 [unused-attribute]


class OtherBase(Exception):
    table_name = {}  # expect: 5 [unused-attribute]
    first: int  # expect: 5 [unused-attribute]

    @register
    def decorated(self):  # expect: 9 [unused-method]
        return self


# A base class that is not a dotted name is not resolved.
class Computed(typing.cast("type", BaseModel)):
    table_name = {}  # expect: 5 [unused-attribute]




# The override does not apply outside the class.
table_name = {}  # expect: 1 [unused-variable]

print(
    Direct,
    ThroughModule,
    ThroughRenamedImport,
    ThroughRenamedModule,
    GrandChild,
    Generic,
    Outer.Inner,
    Row,
    Table,
    Plain,
    OtherBase,
    Computed,
    ThroughNested,
)
