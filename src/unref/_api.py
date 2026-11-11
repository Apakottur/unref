import os
from collections.abc import Iterable

from unref._engine import find_unreferenced
from unref._models import Finding


def check(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_names: Iterable[str] = (),
    used_decorators: Iterable[str] = (),
    exclude: Iterable[str | os.PathLike[str]] = (),
) -> list[Finding]:
    """
    Find the unreferenced code in a Python project.

    Args:
        paths: The files and directories that make up the project.
        used_names: The names of the definitions that count as used, as glob patterns such as `test_*`.
        used_decorators: The decorators that make the definitions they decorate count as used, as
            dotted names. For example, `app.command` covers `@app.command` and `@app.command("name")`.
        exclude: The files and directories that are not part of the project. They are not checked, and
            a reference from them does not count.

    Returns:
        The findings, one for each definition that nothing references, sorted by file and line.
    """
    return find_unreferenced(
        paths,
        used_names=used_names,
        used_decorators=used_decorators,
        exclude=exclude,
    )
