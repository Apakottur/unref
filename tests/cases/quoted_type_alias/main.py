# The value of a type alias is a type, so the names in a quoted part of it are read.
from collections.abc import Mapping, Sequence
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


print(JSONValue, Old, first)
