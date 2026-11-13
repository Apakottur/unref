import unittest

import pytest


# The rules of pytest are turned off.
def test_function():  # expect: 5 [unused-function]
    pass


@pytest.fixture
def plain():  # expect: 5 [unused-function]
    pass


def pytest_configure(config):  # expect: 5 [unused-function]
    pass


class TestClass:  # expect: 7 [unused-class]
    def test_method(self):  # expect: 9 [unused-method] transitive
        pass


# The rules of unittest still apply.
class Case(unittest.TestCase):
    def test_method(self):
        pass
