# The rules of Sphinx are turned off.
project = "Example"  # expect: 1 [unused-variable]
extensions = []  # expect: 1 [unused-variable]


def setup(app):  # expect: 5 [unused-function]
    pass
