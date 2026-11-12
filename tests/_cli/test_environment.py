import shutil
import sys
from pathlib import Path

import pytest
from typer.testing import CliRunner

from unref._cli import app

_runner = CliRunner()

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
)


def _no_config(tmp_path: Path) -> Path:
    """
    A configuration file without options, so that the case does not set its environment itself.
    """
    config = tmp_path / "unref.toml"
    config.write_text("")
    return config


# As with ty, the environment is given by its directory or by its interpreter.
@pytest.mark.parametrize("python", [".venv", ".venv/bin/python"])
def test_option(monkeypatch: pytest.MonkeyPatch, tmp_path: Path, python: str) -> None:
    monkeypatch.chdir(tmp_path)
    monkeypatch.delenv("VIRTUAL_ENV", raising=False)

    result = _runner.invoke(app, ["--config", str(_no_config(tmp_path)), "--python", str(_CASE / python), str(_CASE)])

    assert result.stdout == _WITH_ENVIRONMENT


def test_option_windows_system_python(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    # The interpreter of a system Python on Windows is directly in its `sys.prefix`, next to
    # `Lib/site-packages`.
    prefix = tmp_path / "Python314"
    shutil.copytree(_CASE / ".venv" / "lib" / "python3.14" / "site-packages", prefix / "Lib" / "site-packages")
    (prefix / "python.exe").touch()
    monkeypatch.chdir(tmp_path)
    monkeypatch.delenv("VIRTUAL_ENV", raising=False)

    result = _runner.invoke(
        app, ["--config", str(_no_config(tmp_path)), "--python", str(prefix / "python.exe"), str(_CASE)]
    )

    # The `.pth` file of the copy lists a directory that is not next to it.
    assert result.stdout == (
        f"{_CASE}/main.py:15:9: unused-method 'own_method'\n"
        f"{_CASE}/main.py:21:9: unused-method 'hook'\n"
        f"{_CASE}/main.py:28:9: unused-method 'command'\n"
    )


def test_option_missing(tmp_path: Path) -> None:
    missing = tmp_path / "missing"

    result = _runner.invoke(app, ["--config", str(_no_config(tmp_path)), "--python", str(missing), str(_CASE)])

    assert result.exit_code == 2
    assert result.stderr == f"Error: The Python environment {missing} does not exist\n"


def test_config(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    # As with ruff, the environment of the configuration file is relative to the current directory, not
    # to the configuration file.
    config = tmp_path / "unref.toml"
    config.write_text(f'python = "{_CASE.name}/.venv"\n')
    monkeypatch.chdir(_CASE.parent)
    monkeypatch.delenv("VIRTUAL_ENV", raising=False)

    result = _runner.invoke(app, ["--config", str(config), str(_CASE)])

    assert result.stdout == _WITH_ENVIRONMENT


def test_active_environment(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("VIRTUAL_ENV", str(_CASE / ".venv"))

    result = _runner.invoke(app, ["--config", str(_no_config(tmp_path)), str(_CASE)])

    assert result.stdout == _WITH_ENVIRONMENT


def test_venv_of_current_directory(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    # A prek or pre-commit hook activates the environment that runs the check, which is not the one of
    # the project.
    monkeypatch.chdir(_CASE)
    monkeypatch.setenv("VIRTUAL_ENV", sys.prefix)

    result = _runner.invoke(app, ["--config", str(_no_config(tmp_path)), str(_CASE)])

    assert result.stdout == _WITH_ENVIRONMENT


def test_no_environment(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.chdir(tmp_path)
    monkeypatch.delenv("VIRTUAL_ENV", raising=False)

    result = _runner.invoke(app, ["--config", str(_no_config(tmp_path)), str(_CASE)])

    assert result.stdout == _WITHOUT_ENVIRONMENT
