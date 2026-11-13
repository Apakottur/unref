"""
Check the test frameworks that the library takes.
"""

from pathlib import Path
from typing import Any

import pytest

import unref

_CASES_DIR = Path(__file__).parent / "cases"


def test_unknown_framework() -> None:
    # A name that is not a framework, from a caller without type checks.
    frameworks: dict[Any, bool] = {unref.Framework.PYTEST: True, "nose": False}

    with pytest.raises(
        ValueError, match="Unknown framework 'nose', expected one of 'pytest', 'unittest', 'pydantic', 'sqlalchemy'"
    ):
        unref.check([_CASES_DIR / "function"], frameworks=frameworks)


def test_framework_turned_off() -> None:
    findings = unref.check([_CASES_DIR / "frameworks_selected"], frameworks={unref.Framework.PYTEST: False})

    # The rules of pytest are off, and those of unittest keep their default.
    assert [finding.name for finding in findings] == [
        "test_function",
        "plain",
        "pytest_configure",
        "TestClass",
        "test_method",
    ]
