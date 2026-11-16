import shutil
import warnings
from pathlib import Path

import pytest

from tests._unref import run

_CASES_DIR = Path(__file__).parent.parent / "cases"


def test_no_findings() -> None:
    result = run([str(_CASES_DIR / "empty_file")])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_findings() -> None:
    case = _CASES_DIR / "method"

    result = run([str(case)])

    assert result.exit_code == 1
    # The methods of the unused class are transitive, and not shown by default.
    assert result.stdout == f"{case}/main.py:1:7: unused-class 'Class'\n"


def test_file_path() -> None:
    # Only the given file is checked, not the other files next to it.
    case = _CASES_DIR / "package" / "pkg"

    result = run([str(case / "module.py")])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/module.py:1:5: unused-function 'function'\n"


def test_default_path(tmp_path: Path) -> None:
    # Without arguments and without a `pyproject.toml`, the current directory is checked.
    project = tmp_path / "project"
    shutil.copytree(_CASES_DIR / "package", project)

    result = run([], cwd=project)

    assert result.exit_code == 1
    assert (
        result.stdout == "pkg/module.py:1:5: unused-function 'function'\npkg/sub/module.py:1:7: unused-class 'Class'\n"
    )


def test_gitignored_given_directory(monkeypatch: pytest.MonkeyPatch) -> None:
    # A directory that git ignores is still checked when it is given directly, as with ruff and ty.
    monkeypatch.chdir(_CASES_DIR / "gitignore")

    result = run(["main.py", "generated"])

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

    result = run([str(tmp_path / "ignored" / "function")])

    assert result.exit_code == 1
    assert result.stdout == f"{tmp_path}/ignored/function/main.py:1:5: unused-function 'function'\n"


def test_default_excluded_given_directory(monkeypatch: pytest.MonkeyPatch) -> None:
    # A directory that is excluded by default is still checked when it is given directly.
    monkeypatch.chdir(_CASES_DIR / "default_excluded")

    result = run(["main.py", "node_modules"])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_syntax_error() -> None:
    # A file that does not parse is reported, and the rest of the project is still checked.
    case = _CASES_DIR / "syntax_error"

    result = run([str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/bad.py:7:7: invalid-syntax 'Simple statements must be separated by newlines or semicolons'\n"
        f"{case}/good.py:1:5: unused-function 'unused'\n"
    )


@pytest.mark.parametrize("name", ["missing.py", "missing"])
def test_missing_path(name: str) -> None:
    # A given path that does not exist, whether a file or a directory, is reported as a file that
    # cannot be read, as with ruff, and the rest of the project is still checked.
    case = _CASES_DIR / "method"
    missing = case / name

    result = run([str(missing), str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/main.py:1:7: unused-class 'Class'\n{missing}:1:1: io-error 'No such file or directory (os error 2)'\n"
    )


def test_syntax_warning() -> None:
    # The warnings of parsing, such as for an invalid escape sequence, are not shown. Pytest records the
    # warnings itself, so they are recorded here to see them.
    case = _CASES_DIR / "syntax_warning"

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        result = run([str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:1: unused-variable 'PATTERN'\n"
    assert result.stderr == ""
    assert caught == []


@pytest.mark.parametrize("option", ["--jobs", "-j"])
def test_jobs(option: str) -> None:
    case = _CASES_DIR / "function"

    result = run([option, "1", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


def test_invalid_jobs() -> None:
    result = run(["--jobs", "0", str(_CASES_DIR / "function")])

    assert result.exit_code == 2
    assert result.stdout == ""
    # The message is colored in some terminals, so only a part of it that has no color is checked.
    assert "the number of jobs must be at least 1" in result.stderr


def test_several_threads(tmp_path: Path) -> None:
    # The files are analyzed by several threads at once, which finds the same as one thread.
    for index in range(4):
        shutil.copytree(_CASES_DIR, tmp_path / f"copy_{index}")

    one = run(["--jobs", "1", "--show-transitive", str(tmp_path)])
    several = run(["--jobs", "4", "--show-transitive", str(tmp_path)])

    assert one.exit_code == 1
    assert one.stdout
    assert several.stdout == one.stdout


@pytest.mark.parametrize(
    ("ignore_file", "config"),
    [(".gitignore", "unref.toml"), ("unused.gitignore", "pyproject.toml")],
)
def test_pattern_through_given_directory(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, ignore_file: str, config: str
) -> None:
    # A pattern that matches what is under a directory given directly, only through the directory, does
    # not count, whether git ignores it or it is excluded.
    (tmp_path / ".git").mkdir()
    (tmp_path / ignore_file).write_text("generated/**\n")
    (tmp_path / config).write_text('[tool.unref]\nexclude = ["generated/**"]\n')
    (tmp_path / "generated").mkdir()
    (tmp_path / "generated" / "module.py").write_text("def function():\n    pass\n")
    monkeypatch.chdir(tmp_path)

    given = run(["generated"])
    whole = run([])

    assert given.stdout == "generated/module.py:1:5: unused-function 'function'\n"
    assert whole.stdout == ""
