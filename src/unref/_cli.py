import gc
import importlib.metadata
import os
import sys
from collections.abc import Generator
from contextlib import contextmanager
from pathlib import Path
from typing import Annotated

import typer

from unref._api import check
from unref._config import OutputFormat, UnrefConfigError, load_config
from unref._models import Finding

app = typer.Typer(
    add_completion=False,
    pretty_exceptions_enable=False,
)


def _show_version(value: bool) -> None:  # noqa: FBT001
    """
    Print the version and exit, if `--version` is given.
    """
    if value:
        typer.echo(f"unref {importlib.metadata.version('unref')}")
        raise typer.Exit


@app.command()
def main(
    paths: Annotated[
        list[Path] | None,
        typer.Argument(
            help="Files and directories to check. Defaults to the current directory.",
            show_default=False,
        ),
    ] = None,
    config: Annotated[
        Path | None,
        typer.Option(
            "--config",
            help="Configuration file. Defaults to ./pyproject.toml.",
            show_default=False,
        ),
    ] = None,
    jobs: Annotated[
        int | None,
        typer.Option(
            "--jobs",
            "-j",
            help="Number of processes. Defaults to one for each CPU.",
            min=1,
            show_default=False,
        ),
    ] = None,
    python: Annotated[
        Path | None,
        typer.Option(
            "--python",
            help="Python environment or interpreter of the project. Defaults to the active one, or else ./.venv.",
            show_default=False,
        ),
    ] = None,
    output_format: Annotated[
        OutputFormat | None,
        typer.Option(
            "--output-format",
            help="Output format of the findings. Defaults to concise.",
            show_default=False,
        ),
    ] = None,
    show_transitive: Annotated[
        bool | None,
        typer.Option(
            "--show-transitive/--no-show-transitive",
            help="Also print the unused definitions that only other unused definitions reach.",
            show_default=False,
        ),
    ] = None,
    version: Annotated[  # noqa: ARG001, FBT002
        bool,
        typer.Option(
            "--version",
            help="Print the version and exit.",
            callback=_show_version,
            is_eager=True,
        ),
    ] = False,
) -> None:
    """
    Find unreferenced Python code.

    Exits with a non-zero status if anything is found.
    """
    try:
        options = load_config(config)
    except UnrefConfigError as error:
        typer.echo(f"Error: {error}", err=True)
        raise typer.Exit(code=2) from error

    environment = python or options.python
    if environment is None:
        environment = _find_environment()
    elif not environment.exists():
        typer.echo(f"Error: The Python environment {environment} does not exist", err=True)
        raise typer.Exit(code=2)

    # Parsing creates millions of objects, and the cyclic garbage collector would keep scanning them
    # although none of them is freed before the check ends. Turning it off almost halves the time it
    # takes to parse a large project. It is done here rather than in `check`, because the command owns
    # its process while a caller of `check` may not want its collector turned off.
    with _gc_disabled():
        findings = check(
            paths or [Path()],
            used_names=options.used_names,
            used_decorators=options.used_decorators,
            exclude=options.exclude,
            overrides=options.overrides,
            jobs=jobs,
            python=environment,
            dummy_variable_rgx=options.dummy_variable_rgx,
            entry_points=options.entry_points,
            frameworks=options.frameworks,
        )

    output_format = output_format or options.output_format
    if show_transitive is None:
        show_transitive = options.show_transitive
    for finding in findings:
        if finding.transitive and not show_transitive:
            continue
        typer.echo(_github(finding) if output_format == OutputFormat.GITHUB else finding)

    # A transitive finding is reached from another unused definition, so there are findings exactly when
    # some are printed.
    if findings:
        raise typer.Exit(code=1)


def _github(finding: Finding) -> str:
    """
    Format a finding as a GitHub Actions workflow command, which shows it as an annotation, laid out as
    ruff's `github` output format lays out its diagnostics. A finding has no end, so the command has no
    `endLine` and `endColumn`.

    Returns:
        The workflow command.
    """
    title = _escape_property(f"unref ({finding.code})")
    file = _escape_property(str(finding.path))
    return f"::error title={title},file={file},line={finding.line},col={finding.column}::{_escape_data(str(finding))}"


def _escape_data(text: str) -> str:
    """
    Escape the message of a workflow command, as the `@actions/core` toolkit of GitHub does.

    Returns:
        The escaped message.
    """
    return text.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def _escape_property(text: str) -> str:
    """
    Escape a property of a workflow command, as the `@actions/core` toolkit of GitHub does.

    Returns:
        The escaped property.
    """
    return _escape_data(text).replace(":", "%3A").replace(",", "%2C")


def _find_environment() -> Path | None:
    """
    Find the virtual environment that the project runs in: the active one, or else the `.venv` of the
    current directory.

    A hook of prek or pre-commit runs in an environment of its own, which it activates, so an active
    environment that runs the command is not the one of the project.

    Returns:
        The directory of the environment, or None if there is none.
    """
    active = os.environ.get("VIRTUAL_ENV", "")
    if active and Path(active).resolve() != Path(sys.prefix).resolve():
        return Path(active)
    if Path(".venv").is_dir():
        return Path(".venv")
    return None


@contextmanager
def _gc_disabled() -> Generator[None]:
    """
    Turn off the cyclic garbage collector, and turn it back on afterwards if it was on before.

    Returns:
        A context manager that the collector is off inside of.
    """
    enabled = gc.isenabled()
    gc.disable()
    try:
        yield
    finally:
        if enabled:
            gc.enable()
