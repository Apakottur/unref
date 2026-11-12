import unittest


# A subclass of `unittest.TestCase` is used, wherever it is, and so are its tests.
class Base(unittest.TestCase):
    def test_in_base(self):
        pass

    def helper(self):  # expect: unused method
        pass


# Module setup and teardown run only in a test file.
def setUpModule():  # expect: unused function
    pass
