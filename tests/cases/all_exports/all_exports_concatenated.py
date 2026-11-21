import os

# The names in the lists and tuples that are concatenated in an assignment to `__all__` are listed.
__all__ = ["first"] + ["second"]
__all__ = __all__ + ["added"] + ["added_too"]
__all__ = os.__all__ + ("in_tuple",) + __all__
__all__: list[str] = __all__ + ["annotated"]
__all__ += ["augmented"] + os.__all__
__all__ = [*os.__all__, "unpacked"]


def first():
    return 1


def second():
    return 2


def added():
    return 3


def added_too():
    return 4


def in_tuple():
    return 5


def annotated():
    return 6


def augmented():
    return 7


def unpacked():
    return 8


def internal():  # expect: 5 [unused-function]
    return 9
