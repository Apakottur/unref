import pytest_asyncio
from pytest_asyncio import fixture
from pytest_asyncio.plugin import fixture as plugin_fixture


# A fixture of pytest-asyncio, however it is imported and with any arguments.
@pytest_asyncio.fixture
async def plain():
    pass


@pytest_asyncio.fixture(scope="module")
async def with_arguments():
    pass


@fixture
async def from_import():
    pass


@plugin_fixture
async def from_plugin():
    pass


async def helper():  # expect: 11 [unused-function]
    pass
