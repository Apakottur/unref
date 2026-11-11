import collections.abc
import typing
from typing import NamedTuple

import pydantic
import pydantic as renamed_module
from pydantic import BaseModel
from pydantic import BaseModel as Renamed


class Direct(BaseModel):
    model_config = {}
    field: int  # expect: unused attribute


class ThroughModule(pydantic.BaseModel):
    model_config = {}


class ThroughRenamedImport(Renamed):
    model_config = {}


class ThroughRenamedModule(renamed_module.BaseModel):
    model_config = {}


# A subclass of a class of the project inherits from its base classes.
class Child(Direct):
    model_config = {}


class GrandChild(Child):
    model_config = {}


# The type arguments of a base class are ignored.
class Generic(Direct[int], typing.Generic[int]):
    model_config = {}


# A nested class is a class like any other.
class Outer:
    class Inner(BaseModel):
        model_config = {}


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
    model_config = {}  # expect: unused attribute


class OtherBase(Exception):
    model_config = {}  # expect: unused attribute
    first: int  # expect: unused attribute

    @register
    def decorated(self):  # expect: unused method
        return self


# A base class that is not a dotted name is not resolved.
class Computed(typing.cast("type", BaseModel)):
    model_config = {}  # expect: unused attribute


class ThroughNested(Outer.Inner):
    model_config = {}  # expect: unused attribute


# The override does not apply outside the class.
model_config = {}  # expect: unused variable

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
