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
        f"{case}/main.py:1: unused class 'Class'\n"
        f"{case}/main.py:2: unused method 'method'\n"
        f"{case}/main.py:5: unused method 'async_method'\n"
    )


def test_file_path() -> None:
    # Only the given file is checked, not the other files next to it.
    case = _CASES_DIR / "package" / "pkg"

    result = _runner.invoke(app, [str(case / "module.py")])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/module.py:1: unused function 'function'\n"


def test_default_path(monkeypatch: pytest.MonkeyPatch) -> None:
    # Without arguments, the current directory is checked.
    monkeypatch.chdir(_CASES_DIR / "package")

    result = _runner.invoke(app, [])

    assert result.exit_code == 1
    assert result.stdout == "pkg/module.py:1: unused function 'function'\npkg/sub/module.py:1: unused class 'Class'\n"


def test_syntax_error() -> None:
    # A file that does not parse is reported, and the rest of the project is still checked.
    case = _CASES_DIR / "syntax_error"

    result = _runner.invoke(app, [str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/bad.py:5: syntax error 'Missing parentheses in call to 'print'. Did you mean print(...)?'\n"
        f"{case}/good.py:1: unused function 'unused'\n"
    )


def test_syntax_warning() -> None:
    # The warnings of parsing, such as for an invalid escape sequence, are not shown. Pytest records the
    # warnings itself, so they are recorded here to see them.
    case = _CASES_DIR / "syntax_warning"

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        result = _runner.invoke(app, [str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1: unused variable 'PATTERN'\n"
    assert result.stderr == ""
    assert caught == []
