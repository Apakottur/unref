# A string in a subscript or a union of a type is a type, and the names in it are read.
from typing import Annotated, Callable, Literal, Optional


class InList:
    pass


class InOptional:
    pass


class InUnion:
    pass


class InCallable:
    pass


class InQuoted:
    pass


class InAnnotated:
    pass


# The metadata of `Annotated` and the values of `Literal` are not types.
class InMetadata:  # expect: 7 [unused-class]
    pass


class InLiteral:  # expect: 7 [unused-class]
    pass


def take(
    first: list["InList"],
    second: Optional["InOptional"],
    third: int | "InUnion",
    fourth: Callable[["InCallable"], None],
    fifth: "dict[str, 'InQuoted']",
    sixth: Annotated["InAnnotated", "InMetadata"],
    seventh: Literal["InLiteral"],
) -> None:
    print(first, second, third, fourth, fifth, sixth, seventh)


take([], None, 1, print, {}, 1, "InLiteral")
