# A relative import in a directory without an `__init__.py` goes up through it as through a package.
from ...main import Child


def by_nested_attribute():
    return Child


def unused():  # expect: 5 [unused-function]
    return 2
