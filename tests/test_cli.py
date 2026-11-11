from pathlib import Path

import pytest

from unref import _cli


def test_no_findings(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    (tmp_path / "main.py").write_text("")

    assert _cli.main([str(tmp_path)]) == 0
    assert capsys.readouterr().out == ""


def test_findings(monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]) -> None:
    # The core cannot produce findings yet, so they are faked to pin down the output and exit code.
    monkeypatch.setattr(_cli, "check", lambda _paths: ["first finding", "second finding"])

    assert _cli.main(["."]) == 1
    assert capsys.readouterr().out == "first finding\nsecond finding\n"


def test_default_path(monkeypatch: pytest.MonkeyPatch) -> None:
    checked: list[Path] = []

    def fake_check(paths: list[Path]) -> list[str]:
        checked.extend(paths)
        return []

    monkeypatch.setattr(_cli, "check", fake_check)

    assert _cli.main([]) == 0
    assert checked == [Path()]
