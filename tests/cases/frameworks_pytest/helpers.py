import pytest


# A file that is not a test file holds no tests.
def test_function():  # expect: 5 [unused-function]
    pass


class TestClass:  # expect: 7 [unused-class]
    def test_method(self):  # expect: 9 [unused-method] transitive
        pass


def setup_module():  # expect: 5 [unused-function]
    pass


# A fixture and the hooks of a plugin are used wherever they are.
@pytest.fixture
def shared():
    pass


def pytest_addoption(parser):
    pass


class Plugin:  # expect: 7 [unused-class]
    def pytest_collection_modifyitems(self, items):
        pass


# A decorator with the same name, but that is not `pytest.fixture`.
def fixture(function):
    return function


@fixture
def not_a_fixture():  # expect: 5 [unused-function]
    pass
