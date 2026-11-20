import os

__all__ = ["first"]
__all__.extend(["extended"])
__all__.extend(("extended_tuple",))
__all__.append("appended")
__all__.extend(os.listdir())
__all__.append(os.name)
__all__.remove("removed")


def first():
    return 1


def extended():
    return 2


def extended_tuple():
    return 3


def appended():
    return 4


def removed():  # expect: 5 [unused-function]
    return 5


def internal():  # expect: 5 [unused-function]
    return 6
