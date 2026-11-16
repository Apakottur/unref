import importlib.metadata
import shutil
from pathlib import Path

import pytest

from tests._cli._files import CASES_DIR, write_config
from tests._unref import run


def test_version() -> None:
    # The version is printed without checking anything, as with ruff and ty.
    result = run(["--version", str(CASES_DIR / "function")])

    assert result.exit_code == 0
    assert result.stdout == f"unref {importlib.metadata.version('unref')}\n"


def test_output_format_concise() -> None:
    case = CASES_DIR / "function"

    result = run(["--output-format", "concise", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


def test_output_format_github() -> None:
    case = CASES_DIR / "syntax_error"

    result = run(["--output-format", "github", str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"::error title=unref (invalid-syntax),file={case}/bad.py,line=7,col=7::{case}/bad.py:7:7: "
        "invalid-syntax 'Simple statements must be separated by newlines or semicolons'\n"
        f"::error title=unref (unused-function),file={case}/good.py,line=1,col=5::{case}/good.py:1:5: "
        "unused-function 'unused'\n"
    )


def test_output_format_github_escaped(tmp_path: Path) -> None:
    # A `%` is escaped everywhere, and a `,` or `:` only in the properties, where they separate the
    # properties from each other and from the message.
    case = tmp_path / "a,b:c%"
    shutil.copytree(CASES_DIR / "function", case)
    escaped = str(case).replace("%", "%25").replace(":", "%3A").replace(",", "%2C")

    result = run(["--output-format", "github", str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"::error title=unref (unused-function),file={escaped}/main.py,line=1,col=5::"
        f"{str(case).replace('%', '%25')}/main.py:1:5: unused-function 'function'\n"
    )


def test_output_format_unknown() -> None:
    result = run(["--output-format", "json", str(CASES_DIR / "function")])

    assert result.exit_code == 2
    assert result.stdout == ""
    # The message is colored in some terminals, so only a part of it that has no color is checked.
    assert "[possible values: concise, github]" in result.stderr


def test_output_format_config(tmp_path: Path) -> None:
    config = write_config(tmp_path, 'output-format = "github"\n')
    case = CASES_DIR / "function"

    result = run(["--config", str(config), str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"::error title=unref (unused-function),file={case}/main.py,line=1,col=5::"
        f"{case}/main.py:1:5: unused-function 'function'\n"
    )


def test_output_format_option_over_config(tmp_path: Path) -> None:
    # The command-line option takes precedence over the configuration file, as with ruff.
    config = write_config(tmp_path, 'output-format = "github"\n')
    case = CASES_DIR / "function"

    result = run(["--config", str(config), "--output-format", "concise", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


# The option or the configuration file shows the methods of the unused class, which are reached only from it.
@pytest.mark.parametrize(
    ("option", "content"),
    [pytest.param(["--show-transitive"], "", id="option"), pytest.param([], "show-transitive = true\n", id="config")],
)
def test_show_transitive(tmp_path: Path, option: list[str], content: str) -> None:
    config = write_config(tmp_path, content)
    case = CASES_DIR / "method"

    result = run(["--config", str(config), *option, str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/main.py:1:7: unused-class 'Class'\n"
        f"{case}/main.py:2:9: unused-method 'method'\n"
        f"{case}/main.py:5:15: unused-method 'async_method'\n"
    )


def test_show_transitive_github() -> None:
    case = CASES_DIR / "method"

    result = run(["--output-format", "github", str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"::error title=unref (unused-class),file={case}/main.py,line=1,col=7::{case}/main.py:1:7: unused-class 'Class'\n"
    )


def test_show_transitive_option_over_config(tmp_path: Path) -> None:
    # The command-line option takes precedence over the configuration file, as with ruff.
    config = write_config(tmp_path, "show-transitive = true\n")
    case = CASES_DIR / "method"

    result = run(["--config", str(config), "--no-show-transitive", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:7: unused-class 'Class'\n"
