import pytest


def test_function():
    return 1


@pytest.fixture
def fixture_in_tests():
    return 2


def helper_in_tests():  # expect: 5 [unused-function]
    return 3


# An override adds to the options at the top level.
def everywhere_in_tests():
    return 4


# Only the override of `tests/unit` covers `Test*`.
class TestInTests:  # expect: 7 [unused-class]
    def test_method(self):
        return 5
