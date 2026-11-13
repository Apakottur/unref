import pytest

# pytest reads these variables of a `conftest.py`.
collect_ignore = ["setup.py"]
collect_ignore_glob = ["*_ignore.py"]

OTHER = 1  # expect: 1 [unused-variable]


def pytest_configure(config):
    pass


@pytest.fixture
def database():
    pass


def helper():  # expect: 5 [unused-function]
    pass


# A `conftest.py` holds no tests.
def test_in_conftest():  # expect: 5 [unused-function]
    pass
