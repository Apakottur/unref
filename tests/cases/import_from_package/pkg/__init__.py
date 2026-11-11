# A name that is imported from a package is defined in its `__init__.py`.
def function():
    return 1


def unused():  # expect: unused function
    return 2
