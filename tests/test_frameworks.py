"""
Check the test frameworks that the library takes.
"""

from pathlib import Path

import pytest

import unref

_CASES_DIR = Path(__file__).parent / "cases"


def test_unknown_framework() -> None:
    with pytest.raises(ValueError, match="Unknown framework 'nose', expected one of 'pytest', 'unittest'"):
        unref.check([_CASES_DIR / "function"], frameworks={"pytest": True, "nose": False})
