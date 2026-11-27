import typing
from typing import ClassVar, dataclass_transform

import declarative
from declarative import BaseFilter, Model, record


# A subclass of a class that `dataclass_transform` marks, at any depth, in a package of the environment.
class RoleFilter(BaseFilter):
    codes: list[str]
    limit: int = 10
    kind: ClassVar[str] = "role"  # expect: 5 [unused-attribute]
    plain = 1  # expect: 5 [unused-attribute]

    # Methods are still reported.
    def describe(self):  # expect: 9 [unused-method]
        return self.codes


class AdminFilter(RoleFilter):
    level: int


# A class with a decorator that `dataclass_transform` marks.
@record
class Point:
    x: int


@declarative.record
class Size:
    width: int


# A subclass of a class whose metaclass `dataclass_transform` marks.
class Customer(Model):
    name: str


# The same three, in the project.
@typing.dataclass_transform()
class LocalBase:
    # The marked class itself is not a dataclass.
    registry: dict[str, int]  # expect: 5 [unused-attribute]


class LocalModel(LocalBase):
    id: int


@dataclass_transform()
class LocalMeta(type):
    pass


class WithMeta(metaclass=LocalMeta):
    id: int


class WithInheritedMeta(WithMeta):
    code: str


@dataclass_transform()
def local_record(cls):
    return cls


@local_record
class Event:
    when: float


# Not marked: a plain base class, metaclass and decorator.
def plain_decorator(cls):
    return cls


@plain_decorator
class Plain(type):
    value: int  # expect: 5 [unused-attribute]


class PlainModel(metaclass=Plain):
    value: int  # expect: 5 [unused-attribute]


RoleFilter, AdminFilter, Point, Size, Customer, LocalModel, WithInheritedMeta, Event, PlainModel
