import shutil
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


@pytest.mark.parametrize("option", ["used-names", "used-decorators", "exclude"])
@pytest.mark.parametrize("value", ['"text"', '["text", 1]'])
def test_config_wrong_type(tmp_path: Path, option: str, value: str) -> None:
    config = tmp_path / "unref.toml"
    config.write_text(f"{option} = {value}\n")

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == f"Error: Option '{option}' in the configuration file {config} must be a list of strings\n"


def test_config_exclude_is_relative_to_current_directory(monkeypatch: pytest.MonkeyPatch) -> None:
    # As with ruff, the excluded paths are relative to the current directory, not to the configuration
    # file: `generated` is the one under `pkg`, and the one at the top of the case is checked.
    case = _CASES_DIR / "exclude"
    monkeypatch.chdir(case / "pkg")

    result = _runner.invoke(app, ["--config", str(case / "pyproject.toml"), str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/generated/deep/module.py:1: unused function 'function'\n"
        f"{case}/generated/module.py:4: unused function 'function'\n"
        f"{case}/main.py:1: unused function 'function'\n"
        f"{case}/single.py:1: unused function 'function'\n"
    )


def test_config_exclude_given_file() -> None:
    # A file that is excluded is still checked when it is given directly, as with ruff and ty.
    case = _CASES_DIR / "exclude"

    result = _runner.invoke(app, ["--config", str(case / "pyproject.toml"), str(case / "single.py")])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/single.py:1: unused function 'function'\n"


def test_config_used_names(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('used-names = ["func*"]\n')

    result = _runner.invoke(app, ["--config", str(config), str(_CASES_DIR / "function")])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_config_overrides(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    # As with ruff, the paths of an override are relative to the current directory, not to the
    # configuration file.
    case = _CASES_DIR / "override"
    config = tmp_path / "pyproject.toml"
    shutil.copy(case / "pyproject.toml", config)
    monkeypatch.chdir(case)

    result = _runner.invoke(app, ["--config", str(config), str(case / "tests")])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/tests/test_module.py:13: unused function 'helper_in_tests'\n"
        f"{case}/tests/test_module.py:23: unused class 'TestInTests'\n"
        f"{case}/tests/unit/test_unit.py:6: unused method 'helper'\n"
    )


@pytest.mark.parametrize(
    ("content", "error"),
    [
        (
            'overrides = "tests"',
            "Option 'overrides' in the configuration file {config} must be a list of tables",
        ),
        (
            'overrides = ["tests"]',
            "Option 'overrides' in the configuration file {config} must be a list of tables",
        ),
        (
            '[[overrides]]\nused-names = ["test_*"]',
            "An override in the configuration file {config} needs one of the options 'paths', 'base-classes', 'class-decorators'",
        ),
        (
            '[[overrides]]\nbase-classes = "pydantic.BaseModel"',
            "Option 'base-classes' in an override in the configuration file {config} must be a list of strings",
        ),
        (
            '[[overrides]]\nclass-decorators = "dataclasses.dataclass"',
            "Option 'class-decorators' in an override in the configuration file {config} must be a list of strings",
        ),
        (
            '[[overrides]]\nclass-decorators = ["@dataclasses.dataclass"]',
            "Option 'class-decorators' in an override in the configuration file {config} must list decorators "
            "without the '@': '@dataclasses.dataclass'",
        ),
        (
            '[[overrides]]\npaths = "tests"',
            "Option 'paths' in an override in the configuration file {config} must be a list of strings",
        ),
        (
            '[[overrides]]\npaths = ["tests"]\nused-names = "test_*"',
            "Option 'used-names' in an override in the configuration file {config} must be a list of strings",
        ),
        (
            '[[overrides]]\npaths = ["tests"]\nexclude = ["tests/data"]',
            "Unknown option 'exclude' in an override in the configuration file {config}",
        ),
        (
            '[[overrides]]\npaths = ["tests"]\nused-decorators = ["@pytest.fixture"]',
            "Option 'used-decorators' in an override in the configuration file {config} "
            "must list decorators without the '@': '@pytest.fixture'",
        ),
    ],
)
def test_config_overrides_invalid(tmp_path: Path, content: str, error: str) -> None:
    config = tmp_path / "unref.toml"
    config.write_text(f"{content}\n")

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == f"Error: {error.format(config=config)}\n"


def test_config_python_wrong_type(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('python = [".venv"]\n')

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == f"Error: Option 'python' in the configuration file {config} must be a string\n"


def test_config_dummy_variable_rgx_wrong_type(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('dummy-variable-rgx = ["^_"]\n')

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == (
        f"Error: Option 'dummy-variable-rgx' in the configuration file {config} must be a string\n"
    )


def test_config_dummy_variable_rgx_invalid(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('dummy-variable-rgx = "^(_"\n')

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr.startswith(
        f"Error: Option 'dummy-variable-rgx' in the configuration file {config} is not a valid regular expression: "
    )
