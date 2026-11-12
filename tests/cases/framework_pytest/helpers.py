import pytest


# A file that is not a test file holds no tests.
def test_function():  # expect: unused function
    pass


class TestClass:  # expect: unused class
    def test_method(self):  # expect: unused method
        pass


def setup_module():  # expect: unused function
    pass


# A fixture and the hooks of a plugin are used wherever they are.
@pytest.fixture
def shared():
    pass


def pytest_addoption(parser):
    pass


class Plugin:  # expect: unused class
    def pytest_collection_modifyitems(self, items):
        pass


# A decorator with the same name, but that is not `pytest.fixture`.
def fixture(function):
    return function


@fixture
def not_a_fixture():  # expect: unused function
    pass
