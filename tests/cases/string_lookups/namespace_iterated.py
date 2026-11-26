# Iterating the namespace of the module reads every name of it.
class Base(Exception): ...


class Forbidden(Base): ...


def handler(): ...


LIMIT = 1


def find_exceptions():
    registry = {}
    for obj in list(globals().values()):
        if isinstance(obj, type):
            registry[obj.__name__] = obj
    return registry


find_exceptions()
