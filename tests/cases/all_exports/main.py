# The names in `__all__` are the public interface of the module.
__all__ = ["CONSTANT", "Exported", "exported"]

CONSTANT = 1
INTERNAL = 2  # expect: unused variable


def exported():
    return 3


def internal():  # expect: unused function
    return 4


class Exported:
    pass


class Internal:  # expect: unused class
    pass
