# The first argument of `typing.cast` is a type, so the names in it are read when it is quoted.
import typing
from typing import cast


class OnlyInCast:
    pass


class InTypingCast:
    pass


# The other arguments are values, and a string there is not a type.
class OnlyAsValue:  # expect: 7 [unused-class]
    pass


cast("OnlyInCast", 1), typing.cast("list[InTypingCast]", "OnlyAsValue")
