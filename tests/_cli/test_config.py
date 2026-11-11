from pathlib import Path

import pytest
from typer.testing import CliRunner

from unref._cli import app

_runner = CliRunner()

_CASES_DIR = Path(__file__).parent.parent / "cases"


def test_config_in_current_directory(monkeypatch: pytest.MonkeyPatch) -> None:
    # Without `--config`, the `pyproject.toml` of the current directory is read.
    monkeypatch.chdir(_CASES_DIR / "used_decorator")

    result = _runner.invoke(app, [])

    assert result.exit_code == 1
    assert result.stdout == (
        "main.py:38: unused class 'Class'\n"
        "main.py:45: unused function 'not_listed'\n"
        "main.py:51: unused function 'other_name'\n"
        "main.py:56: unused function 'longer_name'\n"
        "main.py:61: unused function 'shorter_name'\n"
        "main.py:67: unused function 'subscript'\n"
        "main.py:72: unused function 'attribute_of_subscript'\n"
    )


def test_config_file(tmp_path: Path) -> None:
    # A configuration file that is not a `pyproject.toml` has the options at the top level.
    config = tmp_path / "unref.toml"
    config.write_text('used-decorators = ["functools.cache"]\n')

    result = _runner.invoke(app, ["--config", str(config), str(_CASES_DIR / "decorated_function")])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_config_file_pyproject(tmp_path: Path) -> None:
    config = tmp_path / "pyproject.toml"
    config.write_text('[tool.unref]\nused-decorators = ["functools.cache"]\n')

    result = _runner.invoke(app, ["--config", str(config), str(_CASES_DIR / "decorated_function")])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_config_file_pyproject_without_table(tmp_path: Path) -> None:
    config = tmp_path / "pyproject.toml"
    config.write_text('[project]\nname = "project"\n')
    case = _CASES_DIR / "decorated_function"

    result = _runner.invoke(app, ["--config", str(config), str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:5: unused function 'function'\n"


def test_config_file_missing(tmp_path: Path) -> None:
    config = tmp_path / "missing.toml"

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stdout == ""
    assert result.stderr == f"Error: Cannot read the configuration file {config}: No such file or directory\n"


def test_config_file_invalid_toml(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text("used-decorators = [\n")

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr.startswith(f"Error: Invalid TOML in the configuration file {config}: ")


def test_config_unknown_option(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('used-decorator = ["functools.cache"]\n')

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == f"Error: Unknown option 'used-decorator' in the configuration file {config}\n"


def test_config_decorator_with_at_sign(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('used-decorators = ["functools.cache", "@app.command"]\n')

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == (
        f"Error: Option 'used-decorators' in the configuration file {config} "
        "must list decorators without the '@': '@app.command'\n"
    )


@pytest.mark.parametrize("value", ['"functools.cache"', '["functools.cache", 1]'])
def test_config_wrong_type(tmp_path: Path, value: str) -> None:
    config = tmp_path / "unref.toml"
    config.write_text(f"used-decorators = {value}\n")

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == (
        f"Error: Option 'used-decorators' in the configuration file {config} must be a list of strings\n"
    )
