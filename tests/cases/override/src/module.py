import pytest


# An override does not apply outside its paths.
def test_connection():  # expect: unused function
    return 1


@pytest.fixture
def fixture_in_src():  # expect: unused function
    return 2


class TestInSrc:  # expect: unused class
    pass


# The options at the top level apply everywhere.
def everywhere_in_src():
    return 3
