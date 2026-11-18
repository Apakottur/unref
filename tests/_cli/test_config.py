import shutil
from pathlib import Path

import pytest

from tests._cli._files import CASES_DIR, write_config
from tests._unref import run


# A configuration file that is not a `pyproject.toml` has the options at the top level.
@pytest.mark.parametrize(
    ("name", "content"),
    [
        ("unref.toml", 'used-decorators = ["functools.cache"]\n'),
        ("pyproject.toml", '[tool.unref]\nused-decorators = ["functools.cache"]\n'),
    ],
)
def test_config_file(tmp_path: Path, name: str, content: str) -> None:
    config = write_config(tmp_path, content, name)

    result = run(["--config", str(config), str(CASES_DIR / "decorated_function")])

    assert result.exit_code == 0
    assert result.stdout == ""


def test_config_file_pyproject_without_table(tmp_path: Path) -> None:
    config = write_config(tmp_path, '[project]\nname = "project"\n', "pyproject.toml")
    case = CASES_DIR / "decorated_function"

    result = run(["--config", str(config), str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:5:5: unused-function 'function'\n"


def test_config_file_missing(tmp_path: Path) -> None:
    config = tmp_path / "missing.toml"

    result = run(["--config", str(config)])

    assert result.exit_code == 2
    assert result.stdout == ""
    assert result.stderr == f"Error: Cannot read the configuration file {config}: No such file or directory\n"


def test_config_file_not_utf8(tmp_path: Path) -> None:
    # A TOML file is UTF-8.
    config = tmp_path / "unref.toml"
    config.write_bytes(b"# \xe9\n")

    result = run(["--config", str(config)])

    assert result.exit_code == 2
    assert result.stdout == ""
    # The end of the message comes from a library.
    assert result.stderr.startswith(f"Error: Invalid TOML in the configuration file {config}: ")


def _invalid(test_id: str, content: str, error: str, *, name: str = "unref.toml", partial: bool = False) -> object:
    """
    A configuration file that is not valid, and the error it gives, where `{config}` stands for its path.
    A partial error is only the start of the message, whose end comes from a library.
    """
    return pytest.param(name, content, error, partial, id=test_id)


_WRONG_TYPE = "Option '{option}' in the configuration file {{config}} must be {expected}"
_OVERRIDE_WRONG_TYPE = "Option '{option}' in an override in the configuration file {{config}} must be a list of strings"


@pytest.mark.parametrize(
    ("name", "content", "error", "partial"),
    [
        _invalid(
            "invalid-toml", "used-decorators = [", "Invalid TOML in the configuration file {config}: ", partial=True
        ),
        _invalid(
            "unknown-option",
            'used-decorator = ["functools.cache"]',
            "Unknown option 'used-decorator' in the configuration file {config}",
        ),
        *(
            _invalid(
                f"{option}-{test_id}",
                f"{option} = {value}",
                _WRONG_TYPE.format(option=option, expected="a list of strings"),
            )
            for option in ["used-names", "used-decorators", "exclude"]
            for test_id, value in [("string", '"text"'), ("list-with-integer", '["text", 1]')]
        ),
        _invalid(
            "used-decorators-at-sign",
            'used-decorators = ["functools.cache", "@app.command"]',
            "Option 'used-decorators' in the configuration file {config} "
            "must list decorators without the '@': '@app.command'",
        ),
        _invalid("python-list", 'python = [".venv"]', _WRONG_TYPE.format(option="python", expected="a string")),
        _invalid(
            "dummy-variable-rgx-list",
            'dummy-variable-rgx = ["^_"]',
            _WRONG_TYPE.format(option="dummy-variable-rgx", expected="a string"),
        ),
        _invalid(
            "dummy-variable-rgx-invalid",
            'dummy-variable-rgx = "^(_"',
            "Option 'dummy-variable-rgx' in the configuration file {config} is not a valid regular expression: ",
            partial=True,
        ),
        _invalid(
            "output-format-unknown",
            'output-format = "json"',
            _WRONG_TYPE.format(option="output-format", expected="one of 'concise', 'github'"),
        ),
        _invalid(
            "output-format-list",
            'output-format = ["github"]',
            _WRONG_TYPE.format(option="output-format", expected="one of 'concise', 'github'"),
        ),
        _invalid(
            "show-transitive-string",
            'show-transitive = "yes"',
            _WRONG_TYPE.format(option="show-transitive", expected="a boolean"),
        ),
        _invalid(
            "frameworks-list", 'frameworks = ["pytest"]', _WRONG_TYPE.format(option="frameworks", expected="a table")
        ),
        _invalid(
            "frameworks-unknown",
            "[frameworks]\nnose = true",
            "Option 'frameworks' in the configuration file {config} has an unknown framework 'nose', "
            "expected one of 'pytest', 'unittest', 'pydantic', 'dataclasses', 'sqlalchemy', 'fastapi', 'flask', 'sphinx', "
            "'django'",
        ),
        _invalid(
            "frameworks-not-boolean",
            '[frameworks]\npytest = "false"',
            _WRONG_TYPE.format(option="frameworks.pytest", expected="a boolean"),
        ),
        _invalid(
            "frameworks-sphinx-string",
            '[frameworks]\nsphinx = "docs/conf.py"',
            "Option 'frameworks.sphinx' in the configuration file {config} must be a boolean or a table",
        ),
        _invalid(
            "frameworks-sphinx-configuration-string",
            '[frameworks.sphinx]\nconfiguration = "docs/conf.py"',
            "Option 'configuration' in the 'frameworks.sphinx' table of the configuration file {config} must be a "
            "list of strings",
        ),
        _invalid(
            "frameworks-sphinx-unknown-option",
            '[frameworks.sphinx]\nconf = ["docs/conf.py"]',
            "Unknown option 'conf' in the 'frameworks.sphinx' table of the configuration file {config}",
        ),
        _invalid(
            "frameworks-django-string",
            '[frameworks]\ndjango = "mysite/settings.py"',
            "Option 'frameworks.django' in the configuration file {config} must be a boolean or a table",
        ),
        _invalid(
            "frameworks-django-settings-string",
            '[frameworks.django]\nsettings = "mysite/settings.py"',
            "Option 'settings' in the 'frameworks.django' table of the configuration file {config} must be a "
            "list of strings",
        ),
        _invalid(
            "frameworks-django-unknown-option",
            '[frameworks.django]\nsettings-module = "mysite.settings"',
            "Unknown option 'settings-module' in the 'frameworks.django' table of the configuration file {config}",
        ),
        _invalid(
            "overrides-string",
            'overrides = "tests"',
            _WRONG_TYPE.format(option="overrides", expected="a list of tables"),
        ),
        _invalid(
            "overrides-list-of-strings",
            'overrides = ["tests"]',
            _WRONG_TYPE.format(option="overrides", expected="a list of tables"),
        ),
        _invalid(
            "override-without-selector",
            '[[overrides]]\nused-names = ["test_*"]',
            "An override in the configuration file {config} needs one of the options "
            "'paths', 'base-classes', 'class-decorators'",
        ),
        _invalid(
            "override-base-classes-string",
            '[[overrides]]\nbase-classes = "pydantic.BaseModel"',
            _OVERRIDE_WRONG_TYPE.format(option="base-classes"),
        ),
        _invalid(
            "override-class-decorators-string",
            '[[overrides]]\nclass-decorators = "dataclasses.dataclass"',
            _OVERRIDE_WRONG_TYPE.format(option="class-decorators"),
        ),
        _invalid(
            "override-class-decorators-at-sign",
            '[[overrides]]\nclass-decorators = ["@dataclasses.dataclass"]',
            "Option 'class-decorators' in an override in the configuration file {config} must list decorators "
            "without the '@': '@dataclasses.dataclass'",
        ),
        _invalid(
            "override-paths-string",
            '[[overrides]]\npaths = "tests"',
            _OVERRIDE_WRONG_TYPE.format(option="paths"),
        ),
        _invalid(
            "override-used-names-string",
            '[[overrides]]\npaths = ["tests"]\nused-names = "test_*"',
            _OVERRIDE_WRONG_TYPE.format(option="used-names"),
        ),
        _invalid(
            "override-unknown-option",
            '[[overrides]]\npaths = ["tests"]\nexclude = ["tests/data"]',
            "Unknown option 'exclude' in an override in the configuration file {config}",
        ),
        _invalid(
            "override-used-decorators-at-sign",
            '[[overrides]]\npaths = ["tests"]\nused-decorators = ["@pytest.fixture"]',
            "Option 'used-decorators' in an override in the configuration file {config} "
            "must list decorators without the '@': '@pytest.fixture'",
        ),
        _invalid(
            "tool-string",
            'tool = "x"',
            "Table 'tool' in the configuration file {config} must be a table",
            name="pyproject.toml",
        ),
        _invalid(
            "tool-unref-string",
            '[tool]\nunref = "x"',
            "Table 'tool.unref' in the configuration file {config} must be a table",
            name="pyproject.toml",
        ),
        _invalid(
            "project-string",
            'project = "tool"',
            "Table 'project' in the configuration file {config} must be a table",
            name="pyproject.toml",
        ),
        _invalid(
            "project-entry-points-string",
            '[project]\nentry-points = "main:run"',
            "Table 'project.entry-points' in the configuration file {config} must be a table",
            name="pyproject.toml",
        ),
        _invalid(
            "project-scripts-list",
            '[project]\nscripts = ["main:run"]',
            "Table 'project.scripts' in the configuration file {config} must map names to strings",
            name="pyproject.toml",
        ),
        _invalid(
            "project-gui-scripts-table-value",
            '[project.gui-scripts]\ntool = { reference = "main:run" }',
            "Table 'project.gui-scripts' in the configuration file {config} must map names to strings",
            name="pyproject.toml",
        ),
        _invalid(
            "project-entry-points-group-string",
            '[project.entry-points]\ngroup = "main:run"',
            "Table 'project.entry-points.group' in the configuration file {config} must map names to strings",
            name="pyproject.toml",
        ),
    ],
)
def test_config_invalid(tmp_path: Path, name: str, content: str, error: str, partial: bool) -> None:  # noqa: FBT001
    config = write_config(tmp_path, f"{content}\n", name)

    # The run is in the directory of the file, so that nothing else would be checked if it were valid.
    result = run(["--config", str(config)], cwd=tmp_path)

    assert result.exit_code == 2
    assert result.stdout == ""
    expected = f"Error: {error.format(config=config)}"
    if partial:
        assert result.stderr.startswith(expected)
    else:
        assert result.stderr == f"{expected}\n"


def test_config_exclude_is_relative_to_current_directory() -> None:
    # As with ruff, the excluded paths are relative to the current directory, not to the configuration
    # file: `generated` is the one under `pkg`, and the one at the top of the case is checked.
    case = CASES_DIR / "exclude"

    result = run(["--config", str(case / "pyproject.toml"), str(case)], cwd=case / "pkg")

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/generated/deep/module.py:1:5: unused-function 'function'\n"
        f"{case}/generated/module.py:4:5: unused-function 'function'\n"
        f"{case}/main.py:1:5: unused-function 'function'\n"
        f"{case}/single.py:1:5: unused-function 'function'\n"
    )


def test_config_exclude_given_file() -> None:
    # A file that is excluded is still checked when it is given directly, as with ruff and ty.
    case = CASES_DIR / "exclude"

    result = run(["--config", str(case / "pyproject.toml"), str(case / "single.py")])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/single.py:1:5: unused-function 'function'\n"


def test_config_exclude_given_directory(tmp_path: Path) -> None:
    # An excluded directory is still checked when it is given directly, as with ruff and ty, but the
    # patterns that match below it still count.
    config = write_config(tmp_path, 'exclude = ["generated", "**/deep"]\n')

    result = run(["--config", str(config), "generated"], cwd=CASES_DIR / "exclude")

    assert result.exit_code == 1
    assert result.stdout == "generated/module.py:4:5: unused-function 'function'\n"


def test_config_exclude_under_given_directory_parent() -> None:
    # A directory under an excluded one is checked when it is given directly.
    result = run(["--config", "linters/unref.toml", "tests/cases/function"], cwd=CASES_DIR.parent.parent)

    assert result.exit_code == 1
    assert result.stdout == "tests/cases/function/main.py:1:5: unused-function 'function'\n"


def test_config_overrides(tmp_path: Path) -> None:
    # As with ruff, the paths of an override are relative to the current directory, not to the
    # configuration file.
    case = CASES_DIR / "override"
    config = tmp_path / "pyproject.toml"
    shutil.copy(case / "pyproject.toml", config)

    result = run(["--config", str(config), str(case / "tests")], cwd=case)

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/tests/test_module.py:13:5: unused-function 'helper_in_tests'\n"
        f"{case}/tests/test_module.py:23:7: unused-class 'TestInTests'\n"
        f"{case}/tests/unit/test_unit.py:6:9: unused-method 'helper'\n"
    )


def test_entry_points_from_config_file(tmp_path: Path) -> None:
    # A `pyproject.toml` given with `--config` gives the entry points.
    config = write_config(tmp_path, '[project.scripts]\ntool = "pkg.gui:stop"\n', "pyproject.toml")

    result = run(["--config", str(config), str(CASES_DIR / "entry_points" / "pkg" / "gui.py")])

    assert result.exit_code == 1
    assert result.stdout.endswith("gui.py:2:5: unused-function 'start'\n")


def test_entry_points_not_from_other_file(tmp_path: Path) -> None:
    # With another configuration file, the `pyproject.toml` of the current directory is not read.
    config = write_config(tmp_path)

    result = run(["--config", str(config), "main.py"], cwd=CASES_DIR / "entry_points")

    assert result.exit_code == 1
    assert result.stdout == ("main.py:2:5: unused-function 'run'\nmain.py:10:5: unused-function 'unused'\n")


def test_config_found_from_subdirectory() -> None:
    # Without `--config`, the closest `pyproject.toml` with a `[tool.unref]` table is read from the
    # current directory up, as ty finds its project. Its `exclude` and override paths are relative to
    # its directory, it gives the entry points, and its directory is checked.
    case = CASES_DIR / "config_in_parent"

    result = run([], cwd=case / "src")

    assert result.exit_code == 1
    assert result.stdout == "../scripts/tool.py:5:5: unused-function 'unused_tool'\n"


def test_config_found_from_subdirectory_with_path() -> None:
    # The paths that are given are relative to the current directory, the patterns to the project root.
    case = CASES_DIR / "config_in_parent"

    result = run(["."], cwd=case / "src")

    assert result.exit_code == 0
    assert result.stdout == ""


def _copy_config_in_parent(tmp_path: Path) -> Path:
    """
    Copy the case with its configuration in a parent directory, for a test to change it.
    """
    project = tmp_path / "project"
    shutil.copytree(CASES_DIR / "config_in_parent", project)
    return project


def test_config_found_skips_pyproject_without_table(tmp_path: Path) -> None:
    # A closer `pyproject.toml` without a `[tool.unref]` table is not the one of the project.
    project = _copy_config_in_parent(tmp_path)
    write_config(project / "src", '[project]\nname = "inner"\n', "pyproject.toml")

    result = run([], cwd=project / "src" / "app")

    assert result.exit_code == 1
    assert result.stdout == "../../scripts/tool.py:5:5: unused-function 'unused_tool'\n"


def test_config_found_without_table(tmp_path: Path) -> None:
    # Without a `[tool.unref]` table anywhere, the closest `pyproject.toml` is read, for its entry points.
    project = _copy_config_in_parent(tmp_path)
    write_config(project, '[project.scripts]\napp = "app.cli:main"\n', "pyproject.toml")

    result = run([], cwd=project / "src")

    assert result.exit_code == 1
    assert result.stdout == (
        "../scripts/tool.py:1:5: unused-function 'keep_this'\n"
        "../scripts/tool.py:5:5: unused-function 'unused_tool'\n"
        "../src/app/skip.py:1:5: unused-function 'skipped'\n"
    )


def test_config_found_python_relative_to_root(tmp_path: Path) -> None:
    project = tmp_path / "project"
    (project / "src").mkdir(parents=True)
    write_config(project, '[tool.unref]\npython = "missing"\n', "pyproject.toml")

    result = run([], cwd=project / "src", env={"VIRTUAL_ENV": None})

    assert result.exit_code == 2
    assert result.stderr == "Error: The Python environment ../missing does not exist\n"


def test_config_given_from_subdirectory() -> None:
    # With `--config`, the paths are relative to the current directory, as with ruff, which is checked.
    case = CASES_DIR / "config_in_parent"

    result = run(["--config", "../pyproject.toml"], cwd=case / "src")

    assert result.exit_code == 1
    assert result.stdout == "app/skip.py:1:5: unused-function 'skipped'\n"


def test_config_not_found(tmp_path: Path) -> None:
    # Without a `pyproject.toml` anywhere above, nothing is configured and the current directory is checked.
    project = _copy_config_in_parent(tmp_path)
    (project / "pyproject.toml").unlink()

    result = run([], cwd=project / "src")

    assert result.exit_code == 1
    assert result.stdout == "app/cli.py:1:5: unused-function 'main'\napp/skip.py:1:5: unused-function 'skipped'\n"
