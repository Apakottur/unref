import os
from collections.abc import Iterable

from unref._engine import find_unreferenced
from unref._models import Finding


def check(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_decorators: Iterable[str] = (),
) -> list[Finding]:
    """
    Find the unreferenced code in a Python project.

    Args:
        paths: The files and directories that make up the project.
        used_decorators: The decorators that make the definitions they decorate count as used, as
            dotted names. For example, `app.command` covers `@app.command` and `@app.command("name")`.

    Returns:
        The findings, one for each definition that nothing references, sorted by file and line.
    """
    return find_unreferenced(paths, used_decorators=used_decorators)
