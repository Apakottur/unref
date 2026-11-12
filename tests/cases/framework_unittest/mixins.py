import unittest


# unittest runs the tests that a test case inherits, wherever they are defined.
class Mixin:
    def test_inherited(self):
        pass

    def setUp(self):
        pass

    def helper(self):  # expect: unused method
        pass


class WithMixin(Mixin, unittest.TestCase):
    pass


# Not a base class of a test case.
class Unrelated:  # expect: unused class
    def test_method(self):  # expect: unused method
        pass
