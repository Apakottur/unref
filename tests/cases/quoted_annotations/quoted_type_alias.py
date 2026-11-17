# The value of a type alias is a type, so the names in a quoted part of it are read.
from collections.abc import Callable, Mapping, Sequence
from typing import TypeAlias

type JSONArray = Sequence["JSONValue"]
type JSONObject = Mapping[str, "JSONValue"]
type JSONValue = str | int | None | JSONObject | "JSONArray"

Old: TypeAlias = "list[OldItem]"
OldItem = int


class Bound:
    pass


# The bound of a type parameter is a type as well.
def first[T: "Bound"](values: list[T]) -> T:
    return values[0]


class SpecDefault:
    pass


class TupleDefault:
    pass


class Unpacked:
    pass


# The defaults of a `ParamSpec` and of a `TypeVarTuple` are types as well (PEP 696), and so is an
# unpacked annotation of `*args`.
def call[**P = ["SpecDefault"]](function: Callable[P, None]) -> None:
    print(function)


class Row[*Ts = *tuple["TupleDefault"]]:
    pass


def take(*args: *tuple["Unpacked"]) -> None:
    print(args)


print(JSONValue, Old, first, call, Row, take)
