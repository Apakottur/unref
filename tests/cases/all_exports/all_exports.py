# The names in `__all__` are the public interface of the module.
__all__ = ["CONSTANT", "Exported", "exported"]

CONSTANT = 1
INTERNAL = 2  # expect: 1 [unused-variable]


def exported():
    return 3


def internal():  # expect: 5 [unused-function]
    return 4


class Exported:
    pass


class Internal:  # expect: 7 [unused-class]
    pass
