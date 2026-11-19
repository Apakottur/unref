def by_getattr():
    return 1


def by_hasattr():
    return 2


def by_default():
    return 3


def unused():  # expect: 5 [unused-function]
    return 4


def by_set():  # expect: 5 [unused-function]
    return 5


def by_name():  # expect: 5 [unused-function]
    return 6
