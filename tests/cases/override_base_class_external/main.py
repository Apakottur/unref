import enum
import os
from enum import IntEnum, StrEnum

import _socket
import missing_module

import vendored.namespace
from vendored.base import Base
from vendored.broken import Broken
from vendored.namespace.module import Base as NamespaceBase


# A base class from outside the project is followed through the source of its module: `StrEnum`
# inherits from `enum.Enum` through `enum.ReprEnum`.
class Color(StrEnum):
    RED = "red"


class Size(IntEnum):
    SMALL = 1


class Permission(enum.IntFlag):
    READ = 1


# An excluded module is outside the project, and is read like any other module.
class Shade(Base):
    DARK = 1


class FromNamespaceModule(NamespaceBase):
    LIGHT = 1


# A module that is not found, or that has no Python source, is not followed.
class FromMissing(missing_module.Base):
    VALUE = 1  # expect: 5 [unused-attribute]


class FromCompiled(_socket.socket):
    VALUE = 1  # expect: 5 [unused-attribute]


class FromNotPackage(os.path.Base):
    VALUE = 1  # expect: 5 [unused-attribute]


class FromNamespace(vendored.namespace.Base):
    VALUE = 1  # expect: 5 [unused-attribute]


# A module is not a class.
class FromModule(os):
    VALUE = 1  # expect: 5 [unused-attribute]


# A module that does not parse is not followed.
class FromBroken(Broken):
    VALUE = 1  # expect: 5 [unused-attribute]


# Not a subclass of `enum.Enum`.
class Plain(str):
    VALUE = 1  # expect: 5 [unused-attribute]


print(
    Color,
    Size,
    Permission,
    Shade,
    FromNamespaceModule,
    FromMissing,
    FromCompiled,
    FromNotPackage,
    FromNamespace,
    FromModule,
    FromBroken,
    Plain,
)
