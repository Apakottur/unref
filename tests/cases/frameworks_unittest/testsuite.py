# unittest calls these functions of a file whose name starts with `test`.
def setUpModule():
    pass


def tearDownModule():
    pass


def load_tests(loader, tests, pattern):
    return tests


def helper():  # expect: 5 [unused-function]
    pass


# unittest does not call a function that is in a function.
def test_factory():  # expect: 5 [unused-function]
    def setUpModule():  # expect: 9 [unused-function] transitive
        pass

    def load_tests(loader, tests, pattern):  # expect: 9 [unused-function] transitive
        return tests
