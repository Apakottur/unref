import pytest


# pytest runs the tests that a test class inherits, wherever they are defined.
class Base:
    pytestmark = pytest.mark.slow
    other = 1  # expect: unused attribute

    def test_inherited(self):
        pass

    def setup_method(self):
        pass

    def helper(self):  # expect: unused method
        pass


class Grandparent:
    def test_inherited(self):
        pass


class Parent(Grandparent):
    pass


# Not a base class of a test class.
class Unrelated:  # expect: unused class
    def test_method(self):  # expect: unused method
        pass
