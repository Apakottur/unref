import json
import typing
from collections.abc import Mapping, Sequence
from typing import Optional, TypedDict


# A parameter or local variable annotated with a container passed to a function from outside the
# project reads each TypedDict among its type arguments whole, also nested or in a union.
class PairInfo(TypedDict):
    model_filename: str
    trained_timestamp: int


class Listed(TypedDict):
    listed: int


class Nested(TypedDict):
    nested: int


class InTuple(TypedDict):
    in_tuple: int


class Mapped(TypedDict):
    mapped: int


class Local(TypedDict):
    local: int


class Quoted(TypedDict):
    quoted: int


class InUnion(TypedDict):
    in_union: int


# A container that is spread, looped over or passed to a function of the project, or a type argument
# of a type that is not a container, reads no key.
class Spread(TypedDict):
    spread: int  # expect: 5 [unused-attribute]


class Looped(TypedDict):
    looped: int  # expect: 5 [unused-attribute]


class Taken(TypedDict):
    taken: int  # expect: 5 [unused-attribute]


class Called(TypedDict):
    called: int  # expect: 5 [unused-attribute]


def save(pair_dict: dict[str, PairInfo]) -> str:
    return json.dumps(pair_dict)


def dump(
    listed: list[Listed],
    nested: Sequence[dict[str, Nested]],
    in_tuple: tuple[InTuple, ...],
    mapped: typing.Mapping[str, Mapped],
    quoted: "list[Quoted]",
    in_union: Optional[list[InUnion]] | None,
) -> None:
    local: set[Local] = set()
    json.dumps(listed), json.dumps(nested), json.dumps(in_tuple), json.dumps(mapped)
    json.dumps(quoted), json.dumps(in_union), json.dumps(local)


def take(value: object) -> None:
    value


def other(
    spread: Mapping[str, Spread],
    looped: list[Looped],
    taken: list[Taken],
    called: typing.Callable[[Called], None],
) -> None:
    {**spread}
    for item in looped:
        item
    take(taken)
    json.dumps(called)


save({})
dump([], [], (), {}, [], None)
other({}, [], [], take)
