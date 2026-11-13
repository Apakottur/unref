import pytest


# pytest runs the tests that a test class inherits, wherever they are defined.
class Base:
    pytestmark = pytest.mark.slow
    other = 1  # expect: 5 [unused-attribute]

    def test_inherited(self):
        pass

    def setup_method(self):
        pass

    def helper(self):  # expect: 9 [unused-method]
        pass


class Grandparent:
    def test_inherited(self):
        pass


class Parent(Grandparent):
    pass


# Not a base class of a test class.
class Unrelated:  # expect: 7 [unused-class]
    def test_method(self):  # expect: 9 [unused-method]
        pass
