from pathlib import Path
from typing import Annotated

import typer

from unref._api import check

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
) -> None:
    """
    Find unreferenced Python code.
    """
    check(paths or [Path()])
