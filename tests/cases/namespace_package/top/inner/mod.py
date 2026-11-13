# A directory without an `__init__.py` and without a package above it is on the import path, so this
# module is imported as `mod` and not as `top.inner.mod`.
def by_name():
    return 1


def by_dotted_name():  # expect: 5 [unused-function]
    return 2
