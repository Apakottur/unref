from pathlib import Path

import pytest


@pytest.fixture(autouse=True)
def _empty_current_directory(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """
    Run each test from an empty directory, so that the `pyproject.toml` of this repo is not read as
    the configuration.
    """
    monkeypatch.chdir(tmp_path)
