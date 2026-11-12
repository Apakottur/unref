from . import sibling


def in_submodule():
    return sibling.relative()


def in_package():  # expect: unused function
    return 2
