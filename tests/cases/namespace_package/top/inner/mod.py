# A directory without an `__init__.py` and without a package above it is on the import path, so this
# module is imported as `mod`. It is also imported as `top.inner.mod` from the project root.
def by_name():
    return 1


def by_dotted_name():
    return 2


def by_pattern():
    return 3


def unused():  # expect: 5 [unused-function]
    return 4
