def in_package():
    return 1


def from_sub():
    return 2


def unused():  # expect: 5 [unused-function]
    return 3
