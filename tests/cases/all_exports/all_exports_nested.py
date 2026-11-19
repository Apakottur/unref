# Only an `__all__` at the top level of the module is its public interface.
def in_class():  # expect: 5 [unused-function]
    return 1


def in_class_added():  # expect: 5 [unused-function]
    return 2


def in_function():  # expect: 5 [unused-function]
    return 3


def in_function_annotated():  # expect: 5 [unused-function]
    return 4


class Holder:
    __all__ = ["in_class"]
    __all__ += ["in_class_added"]


def function():
    __all__ = ["in_function"]
    __all__: list[str] = ["in_function_annotated"]


Holder, function
