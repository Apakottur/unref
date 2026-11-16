import shutil
import sys
from pathlib import Path

import pytest

from tests._cli._files import write_config
from tests._unref import run

_CASE = Path(__file__).parent.parent / "cases" / "external_environment"

# The findings with the environment of the case, and without it, which is when the check only looks in
# its own environment.
_WITH_ENVIRONMENT = f"{_CASE}/main.py:15:9: unused-method 'own_method'\n{_CASE}/main.py:28:9: unused-method 'command'\n"
_WITHOUT_ENVIRONMENT = (
    f"{_CASE}/main.py:9:5: unused-attribute 'impl'\n"
    f"{_CASE}/main.py:10:5: unused-attribute 'cache_ok'\n"
    f"{_CASE}/main.py:12:9: unused-method 'process_bind_param'\n"
    f"{_CASE}/main.py:15:9: unused-method 'own_method'\n"
    f"{_CASE}/main.py:21:9: unused-method 'hook'\n"
    f"{_CASE}/main.py:28:9: unused-method 'command'\n"
)

# Removes the active environment for the run, so that the environment of the tests does not count. The
# runs give a configuration file without options, so that the case does not set its environment itself.
_NO_ACTIVE_ENVIRONMENT: dict[str, str | None] = {"VIRTUAL_ENV": None}


# As with ty, the environment is given by its directory or by its interpreter.
@pytest.mark.parametrize("python", [".venv", ".venv/bin/python"])
def test_option(tmp_path: Path, python: str) -> None:
    result = run(
        ["--config", str(write_config(tmp_path)), "--python", str(_CASE / python), str(_CASE)],
        env=_NO_ACTIVE_ENVIRONMENT,
        cwd=tmp_path,
    )

    assert result.stdout == _WITH_ENVIRONMENT


def test_option_windows_system_python(tmp_path: Path) -> None:
    # The interpreter of a system Python on Windows is directly in its `sys.prefix`, next to
    # `Lib/site-packages`.
    prefix = tmp_path / "Python315"
    shutil.copytree(_CASE / ".venv" / "lib" / "python3.15" / "site-packages", prefix / "Lib" / "site-packages")
    (prefix / "python.exe").touch()

    result = run(
        ["--config", str(write_config(tmp_path)), "--python", str(prefix / "python.exe"), str(_CASE)],
        env=_NO_ACTIVE_ENVIRONMENT,
        cwd=tmp_path,
    )

    # The `.pth` file of the copy lists a directory that is not next to it.
    assert result.stdout == (
        f"{_CASE}/main.py:15:9: unused-method 'own_method'\n"
        f"{_CASE}/main.py:21:9: unused-method 'hook'\n"
        f"{_CASE}/main.py:28:9: unused-method 'command'\n"
    )


def test_option_missing(tmp_path: Path) -> None:
    missing = tmp_path / "missing"

    result = run(["--config", str(write_config(tmp_path)), "--python", str(missing), str(_CASE)])

    assert result.exit_code == 2
    assert result.stderr == f"Error: The Python environment {missing} does not exist\n"


def test_config(tmp_path: Path) -> None:
    # As with ruff, the environment of the configuration file is relative to the current directory, not
    # to the configuration file.
    config = write_config(tmp_path, f'python = "{_CASE.name}/.venv"\n')

    result = run(["--config", str(config), str(_CASE)], env=_NO_ACTIVE_ENVIRONMENT, cwd=_CASE.parent)

    assert result.stdout == _WITH_ENVIRONMENT


def test_active_environment(tmp_path: Path) -> None:
    result = run(
        ["--config", str(write_config(tmp_path)), str(_CASE)], env={"VIRTUAL_ENV": str(_CASE / ".venv")}, cwd=tmp_path
    )

    assert result.stdout == _WITH_ENVIRONMENT


def test_venv_of_current_directory(tmp_path: Path) -> None:
    # A prek or pre-commit hook activates the environment that runs the check, which is not the one of
    # the project.
    result = run(["--config", str(write_config(tmp_path)), str(_CASE)], env={"VIRTUAL_ENV": sys.prefix}, cwd=_CASE)

    assert result.stdout == _WITH_ENVIRONMENT


def test_no_environment(tmp_path: Path) -> None:
    result = run(["--config", str(write_config(tmp_path)), str(_CASE)], env=_NO_ACTIVE_ENVIRONMENT, cwd=tmp_path)

    assert result.stdout == _WITHOUT_ENVIRONMENT
