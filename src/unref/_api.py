import os
from collections.abc import Iterable

from unref._engine import find_unreferenced
from unref._models import Finding


def check(paths: Iterable[str | os.PathLike[str]]) -> list[Finding]:
    """
    Find the unreferenced code in a Python project.

    Args:
        paths: The files and directories that make up the project.

    Returns:
        The findings, one for each definition that nothing references, sorted by file and line.
    """
    return find_unreferenced(paths)
