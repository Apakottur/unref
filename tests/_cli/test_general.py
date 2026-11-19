import os
import shutil
import warnings
from pathlib import Path

import pytest

from tests._cli._files import CASES_DIR, write_config
from tests._unref import run


def test_findings() -> None:
    case = CASES_DIR / "method"

    result = run([str(case)])

    assert result.exit_code == 1
    # The methods of the unused class are transitive, and not shown by default.
    assert result.stdout == f"{case}/main.py:1:7: unused-class 'Class'\n"


@pytest.mark.parametrize("option", ["--exit-zero", "-e"])
def test_exit_zero(option: str) -> None:
    case = CASES_DIR / "function"

    result = run([option, str(case)])

    # The findings are still printed.
    assert result.exit_code == 0
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


def test_exit_zero_error() -> None:
    # An error still exits with its own status.
    result = run(["--exit-zero", "--jobs", "0", str(CASES_DIR / "function")])

    assert result.exit_code == 2


def test_file_path() -> None:
    # Only the given file is checked, not the other files next to it.
    case = CASES_DIR / "package" / "pkg"

    result = run([str(case / "module.py")])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/module.py:1:5: unused-function 'function'\n"


def test_default_path(tmp_path: Path) -> None:
    # Without arguments and without a `pyproject.toml`, the current directory is checked.
    project = tmp_path / "project"
    shutil.copytree(CASES_DIR / "package", project)

    result = run([], cwd=project)

    assert result.exit_code == 1
    assert (
        result.stdout == "pkg/module.py:1:5: unused-function 'function'\npkg/sub/module.py:1:7: unused-class 'Class'\n"
    )


def test_gitignored_given_directory() -> None:
    # A directory that git ignores is still checked when it is given directly, as with ruff and ty.
    result = run(["main.py", "generated"], cwd=CASES_DIR / "gitignore")

    # `used_by_ignored` is used by the ignored file.
    assert result.exit_code == 1
    assert (
        result.stdout == "main.py:5:5: unused-function 'used_by_kept'\nmain.py:9:5: unused-function 'used_by_nested'\n"
    )


def test_gitignored_given_directory_parent(tmp_path: Path) -> None:
    # A directory under one that git ignores is checked when it is given directly.
    (tmp_path / ".git").mkdir()
    (tmp_path / ".gitignore").write_text("/ignored\n")
    shutil.copytree(CASES_DIR / "function", tmp_path / "ignored" / "function")

    result = run([str(tmp_path / "ignored" / "function")])

    assert result.exit_code == 1
    assert result.stdout == f"{tmp_path}/ignored/function/main.py:1:5: unused-function 'function'\n"


def test_default_excluded_given_directory() -> None:
    # A directory that is excluded by default is still checked when it is given directly.
    result = run(["main.py", "node_modules"], cwd=CASES_DIR / "default_excluded")

    assert result.exit_code == 0
    assert result.stdout == ""


def test_syntax_error() -> None:
    # A file that does not parse is reported, and the rest of the project is still checked.
    case = CASES_DIR / "syntax_error"

    result = run([str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/bad.py:7:7: invalid-syntax 'Simple statements must be separated by newlines or semicolons'\n"
        f"{case}/good.py:1:5: unused-function 'unused'\n"
    )


@pytest.mark.parametrize(
    ("name", "error"),
    [
        ("undecodable.txt", "(unicode error) 'euc-jp' codec can't decode the source"),
        ("unknown_encoding.txt", "unknown encoding: unknown-codec"),
    ],
)
def test_undecodable(name: str, error: str) -> None:
    # A file that does not decode with the encoding it declares, or that declares an encoding that does
    # not exist, is a syntax error, as in Python. The files are not named `.py`, so that the cases, which
    # read every `.py` file, do not read them, and a file that is given directly is checked whatever its
    # name.
    path = CASES_DIR / "syntax_error" / name

    result = run([str(path)])

    assert result.exit_code == 1
    assert result.stdout == f"{path}:1:1: invalid-syntax '{error}'\n"


@pytest.mark.parametrize("name", ["missing.py", "missing", "missing/module.py", "missing/../main.py"])
def test_missing_path(name: str) -> None:
    # A given path that does not exist, whether a file or a directory, is reported as a file that
    # cannot be read, as with ruff, and the rest of the project is still checked. A path through a
    # directory that does not exist does not exist either, even when a `..` leads back out of it.
    case = CASES_DIR / "method"
    missing = case / name

    result = run([str(missing), str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/main.py:1:7: unused-class 'Class'\n{missing}:1:1: io-error 'No such file or directory (os error 2)'\n"
    )


@pytest.mark.parametrize(("flags", "exit_code"), [([], 1), (["--exit-zero"], 0)])
def test_show_files_reports_missing_path(tmp_path: Path, flags: list[str], exit_code: int) -> None:
    (tmp_path / "visible.py").touch()

    result = run(["--show-files", *flags, "missing.py", "visible.py", "missing.py"], cwd=tmp_path)

    assert result.exit_code == exit_code
    assert result.stdout == "visible.py\n"
    assert result.stderr.startswith("missing.py:1:1: io-error ")
    assert len(result.stderr.splitlines()) == 1


@pytest.mark.skipif(os.name != "posix", reason="requires POSIX path lookup")
@pytest.mark.parametrize("suffix", ["/", "/."])
def test_show_files_reports_file_given_as_directory(tmp_path: Path, suffix: str) -> None:
    (tmp_path / "visible.py").touch()
    invalid = f"visible.py{suffix}"

    result = run(["--show-files", invalid, "visible.py"], cwd=tmp_path)

    assert result.exit_code == 1
    assert result.stdout == "visible.py\n"
    assert result.stderr.startswith(f"{invalid}:1:1: io-error ")
    assert len(result.stderr.splitlines()) == 1


def test_show_files_respects_force_exclude_for_missing_path(tmp_path: Path) -> None:
    result = run(["--show-files", "--force-exclude", "--exclude", "missing.py", "missing.py"], cwd=tmp_path)

    assert result.exit_code == 0
    assert result.stdout == ""
    assert result.stderr == ""


@pytest.mark.skipif(os.name != "posix", reason="requires POSIX directory permissions")
def test_show_files_reports_path_below_unsearchable_directory(tmp_path: Path) -> None:
    private = tmp_path / "private"
    private.mkdir()
    blocked = private / "nested"
    blocked.mkdir()
    (tmp_path / "visible.py").touch()
    private.chmod(0)
    try:
        try:
            blocked.stat()
        except PermissionError as error:
            diagnostic = f"private/nested:1:1: io-error '{error.strerror} (os error {error.errno})'\n"
        else:
            pytest.skip("the current process can resolve the directory")

        result = run(["--show-files", "private/nested", "visible.py"], cwd=tmp_path)
    finally:
        private.chmod(0o700)

    assert result.exit_code == 1
    assert result.stdout == "visible.py\n"
    assert result.stderr == diagnostic


def test_syntax_warning() -> None:
    # The warnings of parsing, such as for an invalid escape sequence, are not shown. Pytest records the
    # warnings itself, so they are recorded here to see them.
    case = CASES_DIR / "syntax_warning"

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        result = run([str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:1: unused-variable 'PATTERN'\n"
    assert result.stderr == ""
    assert caught == []


def test_jobs_short_option() -> None:
    # Every run of a case passes `--jobs`, so only the short option is left to check.
    case = CASES_DIR / "function"

    result = run(["-j", "1", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


def test_invalid_jobs() -> None:
    result = run(["--jobs", "0", str(CASES_DIR / "function")])

    assert result.exit_code == 2
    assert result.stdout == ""
    # The message is colored in some terminals, so only a part of it that has no color is checked.
    assert "the number of jobs must be at least 1" in result.stderr


def test_several_threads(tmp_path: Path) -> None:
    # The files are analyzed by several threads at once, which finds the same as one thread.
    for index in range(4):
        shutil.copytree(CASES_DIR, tmp_path / f"copy_{index}")

    one = run(["--jobs", "1", "--show-transitive", str(tmp_path)])
    several = run(["--jobs", "4", "--show-transitive", str(tmp_path)])

    assert one.exit_code == 1
    assert one.stdout
    assert several.stdout == one.stdout


@pytest.mark.parametrize(
    ("ignore_file", "config"),
    [(".gitignore", "unref.toml"), ("unused.gitignore", "pyproject.toml")],
)
def test_pattern_through_given_directory(tmp_path: Path, ignore_file: str, config: str) -> None:
    # A pattern that matches what is under a directory given directly, only through the directory, does
    # not count, whether git ignores it or it is excluded.
    (tmp_path / ".git").mkdir()
    (tmp_path / ignore_file).write_text("generated/**\n")
    write_config(tmp_path, '[tool.unref]\nexclude = ["generated/**"]\n', config)
    (tmp_path / "generated").mkdir()
    (tmp_path / "generated" / "module.py").write_text("def function():\n    pass\n")

    given = run(["generated"], cwd=tmp_path)
    whole = run([], cwd=tmp_path)

    assert given.stdout == "generated/module.py:1:5: unused-function 'function'\n"
    assert whole.stdout == ""
