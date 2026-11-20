# A function that declares `__all__` global sets the `__all__` of the module.
def set_all():
    global __all__
    __all__ = ["assigned"]
    __all__.append("appended")


def assigned():
    return 1


def appended():
    return 2


def internal():  # expect: 5 [unused-function]
    return 3


set_all()
