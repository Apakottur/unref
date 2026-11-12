# unittest calls these functions of a file whose name starts with `test`.
def setUpModule():
    pass


def tearDownModule():
    pass


def load_tests(loader, tests, pattern):
    return tests


def helper():  # expect: unused function
    pass
