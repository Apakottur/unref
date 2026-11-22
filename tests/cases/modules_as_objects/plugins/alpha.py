def run():
    return 1


def setup():
    return 2


def teardown():
    return 3


def reset():
    return 4


def unused():  # expect: 5 [unused-function]
    return 5


CONSTANT = 5  # expect: 1 [unused-variable]
