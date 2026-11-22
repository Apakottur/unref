def run():
    return 1


def teardown():
    return 2


def connect():  # expect: 5 [unused-function]
    return 3
