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
        "main.py:38:7: unused-class 'Class'\n"
        "main.py:45:5: unused-function 'not_listed'\n"
        "main.py:51:5: unused-function 'other_name'\n"
        "main.py:56:5: unused-function 'longer_name'\n"
        "main.py:61:5: unused-function 'shorter_name'\n"
        "main.py:67:5: unused-function 'subscript'\n"
        "main.py:72:5: unused-function 'attribute_of_subscript'\n"
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
    assert result.stdout == f"{case}/main.py:5:5: unused-function 'function'\n"


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
        f"{case}/generated/deep/module.py:1:5: unused-function 'function'\n"
        f"{case}/generated/module.py:4:5: unused-function 'function'\n"
        f"{case}/main.py:1:5: unused-function 'function'\n"
        f"{case}/single.py:1:5: unused-function 'function'\n"
    )


def test_config_exclude_given_file() -> None:
    # A file that is excluded is still checked when it is given directly, as with ruff and ty.
    case = _CASES_DIR / "exclude"

    result = _runner.invoke(app, ["--config", str(case / "pyproject.toml"), str(case / "single.py")])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/single.py:1:5: unused-function 'function'\n"


def test_config_exclude_given_directory(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    # An excluded directory is still checked when it is given directly, as with ruff and ty, but the
    # patterns that match below it still count.
    case = _CASES_DIR / "exclude"
    config = tmp_path / "unref.toml"
    config.write_text('exclude = ["generated", "**/deep"]\n')
    monkeypatch.chdir(case)

    result = _runner.invoke(app, ["--config", str(config), "generated"])

    assert result.exit_code == 1
    assert result.stdout == "generated/module.py:4:5: unused-function 'function'\n"


def test_config_exclude_under_given_directory_parent(monkeypatch: pytest.MonkeyPatch) -> None:
    # A directory under an excluded one is checked when it is given directly.
    monkeypatch.chdir(_CASES_DIR.parent.parent)

    result = _runner.invoke(app, ["--config", "linters/unref.toml", "tests/cases/function"])

    assert result.exit_code == 1
    assert result.stdout == "tests/cases/function/main.py:1:5: unused-function 'function'\n"


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
        f"{case}/tests/test_module.py:13:5: unused-function 'helper_in_tests'\n"
        f"{case}/tests/test_module.py:23:7: unused-class 'TestInTests'\n"
        f"{case}/tests/unit/test_unit.py:6:9: unused-method 'helper'\n"
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


def test_entry_points_from_config_file(tmp_path: Path) -> None:
    # A `pyproject.toml` given with `--config` gives the entry points.
    config = tmp_path / "pyproject.toml"
    config.write_text('[project.scripts]\ntool = "pkg.gui:stop"\n')

    result = _runner.invoke(app, ["--config", str(config), str(_CASES_DIR / "entry_points" / "pkg" / "gui.py")])

    assert result.exit_code == 1
    assert result.stdout.endswith("gui.py:2:5: unused-function 'start'\n")


def test_entry_points_not_from_other_file(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    # With another configuration file, the `pyproject.toml` of the current directory is not read.
    config = tmp_path / "unref.toml"
    config.write_text("")
    monkeypatch.chdir(_CASES_DIR / "entry_points")

    result = _runner.invoke(app, ["--config", str(config), "main.py"])

    assert result.exit_code == 1
    assert result.stdout == (
        "main.py:2:5: unused-function 'run'\nmain.py:6:5: unused-function 'helper'\nmain.py:10:5: unused-function 'unused'\n"
    )


@pytest.mark.parametrize(
    ("content", "error"),
    [
        ('project = "tool"\n', "Table 'project' in the configuration file {config} must be a table"),
        (
            '[project]\nentry-points = "main:run"\n',
            "Table 'project.entry-points' in the configuration file {config} must be a table",
        ),
        (
            '[project]\nscripts = ["main:run"]\n',
            "Table 'project.scripts' in the configuration file {config} must map names to strings",
        ),
        (
            '[project.gui-scripts]\ntool = { reference = "main:run" }\n',
            "Table 'project.gui-scripts' in the configuration file {config} must map names to strings",
        ),
        (
            '[project.entry-points]\ngroup = "main:run"\n',
            "Table 'project.entry-points.group' in the configuration file {config} must map names to strings",
        ),
    ],
)
def test_entry_points_invalid(tmp_path: Path, content: str, error: str) -> None:
    config = tmp_path / "pyproject.toml"
    config.write_text(content)

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == f"Error: {error.format(config=config)}\n"


def test_config_frameworks_wrong_type(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('frameworks = "pytest"\n')

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == (
        f"Error: Option 'frameworks' in the configuration file {config} must be a list of strings\n"
    )


def test_config_frameworks_unknown(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('frameworks = ["pytest", "nose"]\n')

    result = _runner.invoke(app, ["--config", str(config)])

    assert result.exit_code == 2
    assert result.stderr == (
        f"Error: Option 'frameworks' in the configuration file {config} has an unknown framework 'nose', "
        "expected one of 'pytest', 'unittest'\n"
    )
