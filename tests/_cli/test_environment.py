import os
import shutil
from pathlib import Path

import pytest

from tests._cli._files import CASES_DIR, write_config
from tests._unref import UNREF, run

_CASE = CASES_DIR / "external_environment"

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
    # the project. The command is installed in an environment of its own, wherever the one that the
    # tests run is.
    hook = tmp_path / "hook"
    (hook / "bin").mkdir(parents=True)
    binary = Path(shutil.copy2(UNREF, hook / "bin" / UNREF.name))

    result = run(
        ["--config", str(write_config(tmp_path)), str(_CASE)], env={"VIRTUAL_ENV": str(hook)}, cwd=_CASE, binary=binary
    )

    assert result.stdout == _WITH_ENVIRONMENT


def test_no_environment(tmp_path: Path) -> None:
    result = run(["--config", str(write_config(tmp_path)), str(_CASE)], env=_NO_ACTIVE_ENVIRONMENT, cwd=tmp_path)

    assert result.stdout == _WITHOUT_ENVIRONMENT


def test_python_path(tmp_path: Path) -> None:
    # The directories of `PYTHONPATH` that exist are where modules are imported from.
    python_path = os.pathsep.join([str(tmp_path / "missing"), str(_CASE / ".venv" / "src")])

    result = run(
        ["--config", str(write_config(tmp_path)), str(_CASE)],
        env={**_NO_ACTIVE_ENVIRONMENT, "PYTHONPATH": python_path},
        cwd=tmp_path,
    )

    assert result.stdout == _WITHOUT_ENVIRONMENT.replace(f"{_CASE}/main.py:21:9: unused-method 'hook'\n", "")


def test_python_path_before_environment(tmp_path: Path) -> None:
    # As in Python, a package on `PYTHONPATH` comes before the one of the same name that is installed:
    # the `TypeDecorator` there has no base class with members.
    result = run(
        ["--config", str(write_config(tmp_path)), "--python", str(_CASE / ".venv"), str(_CASE)],
        env={**_NO_ACTIVE_ENVIRONMENT, "PYTHONPATH": str(_CASE / ".venv" / "pythonpath")},
        cwd=tmp_path,
    )

    assert result.stdout == _WITHOUT_ENVIRONMENT.replace(f"{_CASE}/main.py:21:9: unused-method 'hook'\n", "")


@pytest.mark.parametrize("key", ["version", "version_info"])
def test_standard_library_version(tmp_path: Path, key: str) -> None:
    # The standard library is the one of the version that the `pyvenv.cfg` of the environment names,
    # under the Python that it is made from, and not the newest one there.
    case = CASES_DIR / "standard_library"
    environment = tmp_path / "venv"
    environment.mkdir()
    (environment / "pyvenv.cfg").write_text(f"home = {case / '.venv' / 'bin'}\n{key} = 3.13.1\n")

    result = run(
        ["--config", str(write_config(tmp_path)), "--python", str(environment), str(case)],
        env=_NO_ACTIVE_ENVIRONMENT,
        cwd=tmp_path,
    )

    assert result.stdout == f"{case}/main.py:7:9: unused-method 'newest'\n"
