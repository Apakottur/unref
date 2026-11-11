from pathlib import Path

import pytest
from typer.testing import CliRunner

from unref import _cli

_runner = CliRunner()


def test_no_findings(tmp_path: Path) -> None:
    (tmp_path / "main.py").write_text("")

    result = _runner.invoke(_cli.app, [str(tmp_path)])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_findings(monkeypatch: pytest.MonkeyPatch) -> None:
    # The core cannot produce findings yet, so they are faked to pin down the output and exit code.
    monkeypatch.setattr(_cli, "check", lambda _paths: ["first finding", "second finding"])

    result = _runner.invoke(_cli.app, ["."])

    assert result.exit_code == 1
    assert result.stdout == "first finding\nsecond finding\n"


def test_default_path(monkeypatch: pytest.MonkeyPatch) -> None:
    checked: list[Path] = []

    def fake_check(paths: list[Path]) -> list[str]:
        checked.extend(paths)
        return []

    monkeypatch.setattr(_cli, "check", fake_check)

    result = _runner.invoke(_cli.app, [])

    assert result.exit_code == 0
    assert checked == [Path()]
