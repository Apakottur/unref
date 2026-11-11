import importlib.metadata

import unref


def test_package_is_installed() -> None:
    assert unref.__name__ == "unref"
    assert importlib.metadata.version("unref")
