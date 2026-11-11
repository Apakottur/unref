__all__ = ("in_tuple",)
__all__ += ["added"]


def in_tuple():
    return 1


def added():
    return 2


def internal():  # expect: unused function
    return 3
