import pytest


def test_function():
    return 1


@pytest.fixture
def fixture_in_tests():
    return 2


def helper_in_tests():  # expect: unused function
    return 3


# An override adds to the options at the top level.
def everywhere_in_tests():
    return 4


# Only the override of `tests/unit` covers `Test*`.
class TestInTests:  # expect: unused class
    def test_method(self):
        return 5
