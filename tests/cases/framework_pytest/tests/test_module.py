import pytest
from pytest import fixture
from pytest import fixture as renamed

import pytest as renamed_module

# pytest reads these variables of a test module.
pytestmark = pytest.mark.slow
pytest_plugins = ["plugin"]

OTHER = 1  # expect: unused variable


# A function whose name starts with `test` is a test.
def test_function():
    pass


def testFunction():
    pass


def helper():  # expect: unused function
    pass


# The setup and teardown of the module and of its functions.
def setup_module():
    pass


def teardown_module():
    pass


def setUpModule():
    pass


def tearDownModule():
    pass


def setup_function():
    pass


def teardown_function():
    pass


# A fixture, however `pytest.fixture` is imported and with any arguments.
@pytest.fixture
def plain():
    pass


@pytest.fixture(scope="module")
def with_arguments():
    pass


@fixture
def from_import():
    pass


@renamed
def renamed_import():
    pass


@renamed_module.fixture
def renamed_module_import():
    pass


# A hook.
def pytest_generate_tests(metafunc):
    pass


# A class whose name starts with `Test` holds tests.
class TestClass:
    pytestmark = pytest.mark.slow
    other = 1  # expect: unused attribute

    def test_method(self):
        pass

    def helper(self):  # expect: unused method
        pass

    def setup_method(self):
        pass

    def teardown_method(self):
        pass

    @classmethod
    def setup_class(cls):
        pass

    @classmethod
    def teardown_class(cls):
        pass

    # A test class in a test class holds tests too.
    class TestNested:
        def test_nested(self):
            pass

    class Nested:  # expect: unused class
        def test_nested(self):  # expect: unused method
            pass


# Other classes do not hold tests.
class Helper:  # expect: unused class
    def test_method(self):  # expect: unused method
        pass
