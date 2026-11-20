"""
The options that select the files to check: `--exclude`, `--extend-exclude`, `--force-exclude`,
`--respect-gitignore` and `--show-files`.
"""

from pathlib import Path

import pytest

from tests._cli._files import CASES_DIR, write_config
from tests._unref import run

_EXCLUDE = CASES_DIR / "exclude"
_GITIGNORE = CASES_DIR / "gitignore"


def _files(*names: str) -> str:
    """What `--show-files` prints for the given files."""
    return "".join(f"{name}\n" for name in names)


def test_show_files() -> None:
    result = run(["--show-files"], cwd=_EXCLUDE)

    # The files are printed even though some of them have findings.
    assert result.exit_code == 0
    assert result.stdout == _files("main.py", "pkg/generated/module.py")


def test_show_files_given_paths() -> None:
    result = run(["--show-files", "single.py", "pkg"], cwd=_EXCLUDE)

    assert result.exit_code == 0
    assert result.stdout == _files("pkg/generated/module.py", "single.py")


def test_show_files_missing_path() -> None:
    # A given path that does not exist is reported, as by a check, rather than listed.
    result = run(["--show-files", "missing.py", "single.py"], cwd=_EXCLUDE)

    assert result.exit_code == 1
    assert result.stdout == _files("single.py")
    assert result.stderr == "missing.py:1:1: io-error 'No such file or directory (os error 2)'\n"


def test_show_files_force_excluded_missing_path() -> None:
    # A path that is excluded is left out before it is looked up.
    result = run(["--show-files", "--force-exclude", "--exclude", "missing.py", "missing.py"], cwd=_EXCLUDE)

    assert result.exit_code == 0
    assert result.stdout == ""
    assert result.stderr == ""


@pytest.mark.parametrize(
    "args",
    [
        pytest.param(["--exclude", "pkg"], id="once"),
        pytest.param(["--exclude", "pkg,missing"], id="comma-separated"),
        pytest.param(["--exclude", "pkg", "--exclude", "missing"], id="repeated"),
    ],
)
def test_exclude_replaces_config(args: list[str]) -> None:
    result = run(["--show-files", *args], cwd=_EXCLUDE)

    assert result.exit_code == 0
    assert result.stdout == _files("generated/deep/module.py", "generated/module.py", "main.py", "single.py")


def test_extend_exclude_adds_to_config() -> None:
    result = run(["--show-files", "--extend-exclude", "pkg"], cwd=_EXCLUDE)

    assert result.exit_code == 0
    assert result.stdout == _files("main.py")


def test_exclude_and_extend_exclude() -> None:
    result = run(["--show-files", "--exclude", "generated", "--extend-exclude", "**/deep"], cwd=_EXCLUDE)

    assert result.exit_code == 0
    assert result.stdout == _files("main.py", "pkg/generated/module.py", "single.py")


def test_extend_exclude_negation() -> None:
    # A `!` pattern brings back what the configuration excludes.
    result = run(["--show-files", "--extend-exclude", "!single.py"], cwd=_EXCLUDE)

    assert result.exit_code == 0
    assert result.stdout == _files("main.py", "pkg/generated/module.py", "single.py")


def test_exclude_relative_to_current_directory() -> None:
    # The patterns of the configuration are relative to the project root, those of the command line to
    # the current directory: `generated` is the one under `pkg`.
    result = run(["--show-files", "--extend-exclude", "generated"], cwd=_EXCLUDE / "pkg")

    assert result.exit_code == 0
    assert result.stdout == _files("../main.py")


@pytest.mark.parametrize(
    ("pattern", "cwd", "expected"),
    [
        pytest.param(
            "{root}/pkg",
            _EXCLUDE,
            ["generated/deep/module.py", "generated/module.py", "main.py", "single.py"],
            id="from-root",
        ),
        pytest.param(
            "{root}/pkg/generated/",
            _EXCLUDE / "pkg",
            ["../generated/deep/module.py", "../generated/module.py", "../main.py", "../single.py"],
            id="from-subdirectory",
        ),
    ],
)
def test_exclude_absolute_path(pattern: str, cwd: Path, expected: list[str]) -> None:
    # An absolute path under the project root is taken as that path, wherever it is given from.
    result = run(["--show-files", "--exclude", pattern.format(root=_EXCLUDE.resolve())], cwd=cwd)

    assert result.exit_code == 0
    assert result.stdout == _files(*expected)


def test_exclude_references_do_not_count() -> None:
    result = run(["--exclude", "single.py"], cwd=_EXCLUDE)

    # `used_by_excluded` is only used by the excluded `generated/module.py`.
    assert result.exit_code == 1
    assert result.stdout == (
        "generated/deep/module.py:1:5: unused-function 'function'\n"
        "generated/module.py:4:5: unused-function 'function'\n"
        "main.py:1:5: unused-function 'function'\n"
        "pkg/generated/module.py:2:5: unused-function 'nested'\n"
    )


@pytest.mark.parametrize(
    ("args", "content"),
    [
        pytest.param(["--force-exclude"], "", id="option"),
        pytest.param([], "force-exclude = true\n", id="config"),
    ],
)
def test_force_exclude(tmp_path: Path, args: list[str], content: str) -> None:
    config = write_config(tmp_path, 'exclude = ["generated", "single.py"]\n' + content)

    result = run(
        ["--config", str(config), "--show-files", *args, "single.py", "generated/deep", "main.py"], cwd=_EXCLUDE
    )

    # `generated/deep` is left out through the excluded directory above it.
    assert result.exit_code == 0
    assert result.stdout == _files("main.py")


def test_no_force_exclude_over_config(tmp_path: Path) -> None:
    config = write_config(tmp_path, 'exclude = ["single.py"]\nforce-exclude = true\n')

    result = run(["--config", str(config), "--show-files", "--no-force-exclude", "single.py"], cwd=_EXCLUDE)

    assert result.exit_code == 0
    assert result.stdout == _files("single.py")


def test_force_exclude_gitignored(tmp_path: Path) -> None:
    config = write_config(tmp_path)

    result = run(
        [
            "--config",
            str(config),
            "--show-files",
            "--force-exclude",
            "generated",
            "scratch_one.py",
            "scratch_kept.py",
            "nested/scratch_nested.py",
        ],
        cwd=_GITIGNORE,
    )

    assert result.exit_code == 0
    assert result.stdout == _files("nested/scratch_nested.py", "scratch_kept.py")


def test_force_exclude_default_excluded() -> None:
    result = run(["--show-files", "--force-exclude", "main.py", "node_modules"], cwd=CASES_DIR / "default_excluded")

    assert result.exit_code == 0
    assert result.stdout == _files("main.py")


@pytest.mark.parametrize(
    ("args", "content"),
    [
        pytest.param(["--no-respect-gitignore"], "", id="option"),
        pytest.param([], "respect-gitignore = false\n", id="config"),
    ],
)
def test_no_respect_gitignore(tmp_path: Path, args: list[str], content: str) -> None:
    config = write_config(tmp_path, content)

    result = run(["--config", str(config), "--show-files", *args], cwd=_GITIGNORE)

    assert result.exit_code == 0
    assert result.stdout == _files(
        "generated/use.py", "main.py", "nested/scratch_nested.py", "scratch_kept.py", "scratch_one.py"
    )


def test_respect_gitignore_over_config(tmp_path: Path) -> None:
    config = write_config(tmp_path, "respect-gitignore = false\n")

    result = run(["--config", str(config), "--show-files", "--respect-gitignore"], cwd=_GITIGNORE)

    assert result.exit_code == 0
    assert result.stdout == _files("main.py", "nested/scratch_nested.py", "scratch_kept.py")


def test_force_exclude_without_respect_gitignore(tmp_path: Path) -> None:
    config = write_config(tmp_path)

    result = run(
        ["--config", str(config), "--show-files", "--force-exclude", "--no-respect-gitignore", "scratch_one.py"],
        cwd=_GITIGNORE,
    )

    assert result.exit_code == 0
    assert result.stdout == _files("scratch_one.py")
