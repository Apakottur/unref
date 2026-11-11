from pathlib import Path

import pytest
from typer.testing import CliRunner

from unref._cli import app

_runner = CliRunner()


def test_no_findings(tmp_path: Path) -> None:
    (tmp_path / "main.py").write_text("")

    result = _runner.invoke(app, [str(tmp_path)])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_default_path(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    # Without arguments, the current directory is checked.
    (tmp_path / "main.py").write_text("")
    monkeypatch.chdir(tmp_path)

    result = _runner.invoke(app, [])

    assert result.exit_code == 0
    assert result.stdout == ""
