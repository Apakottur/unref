def function():
    return 1


def unused():  # expect: 5 [unused-function]
    return 2


class Class:
    pass
