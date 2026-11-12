import unittest

import pytest


# The rules of pytest are turned off.
def test_function():  # expect: unused function
    pass


@pytest.fixture
def plain():  # expect: unused function
    pass


def pytest_configure(config):  # expect: unused function
    pass


class TestClass:  # expect: unused class
    def test_method(self):  # expect: unused method
        pass


# The rules of unittest still apply.
class Case(unittest.TestCase):
    def test_method(self):
        pass
