import dataclasses
import typing
from dataclasses import KW_ONLY, InitVar, dataclass, field
from typing import ClassVar, NamedTuple

import attr
import attrs
import pydantic.dataclasses
import typing_extensions


# The methods that a dataclass generates read its fields.
@dataclass
class Point:
    x: int
    y: int = 0
    tags: list[str] = field(default_factory=list)
    _: KW_ONLY
    label: "str" = ""
    scale: InitVar[int] = 1

    # Not fields: class variables and names without an annotation.
    origin: ClassVar["Point"]  # expect: 5 [unused-attribute]
    count: typing.ClassVar[int] = 0  # expect: 5 [unused-attribute]
    plain = 1  # expect: 5 [unused-attribute]

    # Methods are still reported.
    def norm(self):  # expect: 9 [unused-method]
        return self.x


# With arguments, frozen, and through the module.
@dataclass(frozen=True, slots=True)
class Frozen:
    name: str


@dataclasses.dataclass(order=True)
class Ordered:
    rank: int


# A dataclass of Pydantic.
@pydantic.dataclasses.dataclass
class Validated:
    email: str


# A named tuple, through `typing` and `typing_extensions`.
class Pair(NamedTuple):
    first: int
    second: int = 0
    kind: ClassVar[str] = "pair"  # expect: 5 [unused-attribute]


class Triple(typing_extensions.NamedTuple):
    third: int


# The classes of attrs, with and without annotations.
@attrs.define
class Modern:
    size: int
    weight = attrs.field(default=0)
    height: int = attrs.field()

    # attrs calls the validators and defaults of a field.
    @height.validator
    def check_height(self, attribute, value):
        return value

    @height.default
    def make_height(self):
        return 0


@attrs.frozen(kw_only=True)
class FrozenModern:
    size: int


@attrs.mutable
class Mutable:
    size: int


@attr.s
class Classic:
    size = attr.ib()
    weight = attr.attrib(default=0)
    other = attr.Factory(list)  # expect: 5 [unused-attribute]


@attr.s(auto_attribs=True)
class AutoClassic:
    size: int


@attr.define
class Next:
    size: int


@attr.frozen
class FrozenNext:
    size: int


# A subclass that is a dataclass has fields of its own, and one that is not has none.
@dataclass
class Point3D(Point):
    z: int = 0


class Unmarked(Point):
    w: int = 0  # expect: 5 [unused-attribute]


# Not a dataclass.
class Plain:
    title: str = ""  # expect: 5 [unused-attribute]


Point, Frozen, Ordered, Validated, Pair, Triple, Modern, FrozenModern, Mutable, Classic, AutoClassic
Next, FrozenNext, Point3D, Unmarked, Plain
