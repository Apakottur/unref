# The bound and the constraints of a `TypeVar` are types, so the names in them are read when they are
# quoted.
from typing import NewType, TypeVar


class Bound:
    pass


class First:
    pass


class Second:
    pass


class Default:
    pass


class Base:
    pass


# The name of a `TypeVar` is a string that is not a type.
class Named:  # expect: 7 [unused-class]
    pass


Bounded = TypeVar("Bounded", bound="Bound", default="Default")
Constrained = TypeVar("Constrained", "First", "Second")
Derived = NewType("Derived", "Base")
Other = TypeVar("Named")

Bounded, Constrained, Derived, Other
