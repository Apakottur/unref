import gc
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
            help="Files and directories that make up the project. Defaults to the current directory.",
            show_default=False,
        ),
    ] = None,
    config: Annotated[
        Path | None,
        typer.Option(
            "--config",
            help="Read the configuration from this TOML file, instead of the pyproject.toml of the current directory.",
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
        )

    for finding in findings:
        typer.echo(finding)

    if findings:
        raise typer.Exit(code=1)


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
