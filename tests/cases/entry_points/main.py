# The object of an entry point is used, and so is what it uses.
def run():
    return helper()


def helper():
    return 1


def unused():  # expect: 5 [unused-function]
    return 2
