import gc
import os
import sys
from collections.abc import Generator
from contextlib import contextmanager
from pathlib import Path
from typing import Annotated

import typer

from unref._api import check
from unref._config import UnrefConfigError, load_config

app = typer.Typer(
    add_completion=False,
    pretty_exceptions_enable=False,
)


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
        )

    for finding in findings:
        typer.echo(finding)

    if findings:
        raise typer.Exit(code=1)


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
