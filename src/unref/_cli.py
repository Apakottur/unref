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

    findings = check(paths or [Path()], used_decorators=options.used_decorators)

    for finding in findings:
        typer.echo(finding)

    if findings:
        raise typer.Exit(code=1)
