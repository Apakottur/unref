# Only an `__all__` at the top level of the module is its public interface.
def in_class():  # expect: 5 [unused-function]
    return 1


def in_class_added():  # expect: 5 [unused-function]
    return 2


def in_function():  # expect: 5 [unused-function]
    return 3


def in_function_annotated():  # expect: 5 [unused-function]
    return 4


def in_class_extended():  # expect: 5 [unused-function]
    return 5


def in_function_appended():  # expect: 5 [unused-function]
    return 6


class Holder:
    __all__ = ["in_class"]
    __all__ += ["in_class_added"]
    __all__.extend(["in_class_extended"])


def function():
    __all__ = ["in_function"]
    __all__: list[str] = ["in_function_annotated"]
    __all__.append("in_function_appended")


Holder, function
