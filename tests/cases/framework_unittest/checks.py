import unittest
from unittest import TestCase
from unittest import TestCase as Renamed

from base import Base


class Direct(unittest.TestCase):
    def test_method(self):
        pass

    def testMethod(self):
        pass

    def helper(self):  # expect: unused method
        pass

    def setUp(self):
        pass

    def tearDown(self):
        pass

    @classmethod
    def setUpClass(cls):
        pass

    @classmethod
    def tearDownClass(cls):
        pass

    class Nested(TestCase):
        def test_nested(self):
            pass


class FromImport(TestCase):
    def test_method(self):
        pass


class FromRenamedImport(Renamed):
    def test_method(self):
        pass


# Through a subclass of the project.
class FromProject(Base):
    def test_method(self):
        pass


# Through a subclass outside the project.
class Async(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        pass

    async def asyncTearDown(self):
        pass

    async def test_method(self):
        pass


# Not a test case.
class Plain:  # expect: unused class
    def test_method(self):  # expect: unused method
        pass
