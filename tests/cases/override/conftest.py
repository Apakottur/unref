import pytest


# A path of an override can be a file.
@pytest.fixture
def fixture_in_conftest():
    return 1


def helper_in_conftest():  # expect: 5 [unused-function]
    return 2
