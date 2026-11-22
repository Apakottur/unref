def run():  # expect: 5 [unused-function]
    return 1


def setup():  # expect: 5 [unused-function]
    return 2


def connect():  # expect: 5 [unused-function]
    return 3


def handle():  # expect: 5 [unused-function]
    return 4


def start():  # expect: 5 [unused-function]
    return 5


CONSTANT = 6  # expect: 1 [unused-variable]
