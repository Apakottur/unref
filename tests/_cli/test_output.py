import importlib.metadata
import shutil
from pathlib import Path

import pytest
from typer.testing import CliRunner

from unref._cli import app

_runner = CliRunner()

_CASES_DIR = Path(__file__).parent.parent / "cases"


def test_version() -> None:
    # The version is printed without checking anything, as with ruff and ty.
    result = _runner.invoke(app, ["--version", str(_CASES_DIR / "function")])

    assert result.exit_code == 0
    assert result.stdout == f"unref {importlib.metadata.version('unref')}\n"


def test_output_format_concise() -> None:
    case = _CASES_DIR / "function"

    result = _runner.invoke(app, ["--output-format", "concise", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


def test_output_format_github() -> None:
    case = _CASES_DIR / "syntax_error"

    result = _runner.invoke(app, ["--output-format", "github", str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"::error title=unref (invalid-syntax),file={case}/bad.py,line=6,col=1::{case}/bad.py:6:1: "
        "invalid-syntax 'Missing parentheses in call to 'print'. Did you mean print(...)?'\n"
        f"::error title=unref (unused-function),file={case}/good.py,line=1,col=5::{case}/good.py:1:5: "
        "unused-function 'unused'\n"
    )


def test_output_format_github_escaped(tmp_path: Path) -> None:
    # A `%` is escaped everywhere, and a `,` or `:` only in the properties, where they separate the
    # properties from each other and from the message.
    case = tmp_path / "a,b:c%"
    shutil.copytree(_CASES_DIR / "function", case)
    escaped = str(case).replace("%", "%25").replace(":", "%3A").replace(",", "%2C")

    result = _runner.invoke(app, ["--output-format", "github", str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"::error title=unref (unused-function),file={escaped}/main.py,line=1,col=5::"
        f"{str(case).replace('%', '%25')}/main.py:1:5: unused-function 'function'\n"
    )


def test_output_format_unknown() -> None:
    result = _runner.invoke(app, ["--output-format", "json", str(_CASES_DIR / "function")])

    assert result.exit_code == 2
    assert result.stdout == ""
    # The message is colored in some terminals, so only a part of it that has no color is checked.
    assert "'concise', 'github'" in result.stderr


def test_output_format_config(tmp_path: Path) -> None:
    config = tmp_path / "unref.toml"
    config.write_text('output-format = "github"\n')
    case = _CASES_DIR / "function"

    result = _runner.invoke(app, ["--config", str(config), str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"::error title=unref (unused-function),file={case}/main.py,line=1,col=5::"
        f"{case}/main.py:1:5: unused-function 'function'\n"
    )


def test_output_format_option_over_config(tmp_path: Path) -> None:
    # The command-line option takes precedence over the configuration file, as with ruff.
    config = tmp_path / "unref.toml"
    config.write_text('output-format = "github"\n')
    case = _CASES_DIR / "function"

    result = _runner.invoke(app, ["--config", str(config), "--output-format", "concise", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


@pytest.mark.parametrize("value", ['"json"', '["github"]'])
def test_output_format_config_invalid(tmp_path: Path, value: str) -> None:
    config = tmp_path / "unref.toml"
    config.write_text(f"output-format = {value}\n")

    result = _runner.invoke(app, ["--config", str(config), str(_CASES_DIR / "function")])

    assert result.exit_code == 2
    assert result.stdout == ""
    assert result.stderr == (
        f"Error: Option 'output-format' in the configuration file {config} must be one of 'concise', 'github'\n"
    )
