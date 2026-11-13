import shutil
import warnings
from pathlib import Path

import pytest
from typer.testing import CliRunner

from unref._cli import app

_runner = CliRunner()

_CASES_DIR = Path(__file__).parent.parent / "cases"


def test_no_findings() -> None:
    result = _runner.invoke(app, [str(_CASES_DIR / "empty_file")])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_findings() -> None:
    case = _CASES_DIR / "method"

    result = _runner.invoke(app, [str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/main.py:1:7: unused-class 'Class'\n"
        f"{case}/main.py:2:9: unused-method 'method'\n"
        f"{case}/main.py:5:15: unused-method 'async_method'\n"
    )


def test_file_path() -> None:
    # Only the given file is checked, not the other files next to it.
    case = _CASES_DIR / "package" / "pkg"

    result = _runner.invoke(app, [str(case / "module.py")])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/module.py:1:5: unused-function 'function'\n"


def test_default_path(monkeypatch: pytest.MonkeyPatch) -> None:
    # Without arguments, the current directory is checked.
    monkeypatch.chdir(_CASES_DIR / "package")

    result = _runner.invoke(app, [])

    assert result.exit_code == 1
    assert (
        result.stdout == "pkg/module.py:1:5: unused-function 'function'\npkg/sub/module.py:1:7: unused-class 'Class'\n"
    )


def test_gitignored_given_directory(monkeypatch: pytest.MonkeyPatch) -> None:
    # A directory that git ignores is still checked when it is given directly, as with ruff and ty.
    monkeypatch.chdir(_CASES_DIR / "gitignore")

    result = _runner.invoke(app, ["main.py", "generated"])

    # `used_by_ignored` is used by the ignored file.
    assert result.exit_code == 1
    assert (
        result.stdout == "main.py:5:5: unused-function 'used_by_kept'\nmain.py:9:5: unused-function 'used_by_nested'\n"
    )


def test_gitignored_given_directory_parent(tmp_path: Path) -> None:
    # A directory under one that git ignores is checked when it is given directly.
    (tmp_path / ".git").mkdir()
    (tmp_path / ".gitignore").write_text("/ignored\n")
    shutil.copytree(_CASES_DIR / "function", tmp_path / "ignored" / "function")

    result = _runner.invoke(app, [str(tmp_path / "ignored" / "function")])

    assert result.exit_code == 1
    assert result.stdout == f"{tmp_path}/ignored/function/main.py:1:5: unused-function 'function'\n"


def test_default_excluded_given_directory(monkeypatch: pytest.MonkeyPatch) -> None:
    # A directory that is excluded by default is still checked when it is given directly.
    monkeypatch.chdir(_CASES_DIR / "default_excluded")

    result = _runner.invoke(app, ["main.py", "node_modules"])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_syntax_error() -> None:
    # A file that does not parse is reported, and the rest of the project is still checked.
    case = _CASES_DIR / "syntax_error"

    result = _runner.invoke(app, [str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/bad.py:6:1: invalid-syntax 'Missing parentheses in call to 'print'. Did you mean print(...)?'\n"
        f"{case}/good.py:1:5: unused-function 'unused'\n"
    )


def test_syntax_warning() -> None:
    # The warnings of parsing, such as for an invalid escape sequence, are not shown. Pytest records the
    # warnings itself, so they are recorded here to see them.
    case = _CASES_DIR / "syntax_warning"

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        result = _runner.invoke(app, [str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:1: unused-variable 'PATTERN'\n"
    assert result.stderr == ""
    assert caught == []


@pytest.mark.parametrize("option", ["--jobs", "-j"])
def test_jobs(option: str) -> None:
    case = _CASES_DIR / "function"

    result = _runner.invoke(app, [option, "1", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


def test_invalid_jobs() -> None:
    result = _runner.invoke(app, ["--jobs", "0", str(_CASES_DIR / "function")])

    assert result.exit_code == 2
    assert result.stdout == ""
    # The message is colored in some terminals, so only a part of it that has no color is checked.
    assert "greater than or equal to" in result.stderr
