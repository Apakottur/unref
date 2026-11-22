CONSTANT = 1


def describe():
    return 2


def unused():  # expect: 5 [unused-function]
    return 3
