import sys

__all__ = ["removed", "called"]
__all__.append("removed_append")
__all__ = ["kept"]
__all__ += ["added"]
if sys.platform == "win32":
    __all__ = ["conditional"]
__all__ = [*__all__, "unpacked"]


def removed():  # expect: 5 [unused-function]
    return 1


def called():
    return 2


def removed_append():  # expect: 5 [unused-function]
    return 3


def kept():
    return 4


def added():
    return 5


def conditional():
    return 6


def unpacked():
    return 7


called()
