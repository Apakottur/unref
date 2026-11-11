import os
from collections.abc import Iterable

from unref._definitions import find_definitions
from unref._discover import discover
from unref._models import Finding


def check(paths: Iterable[str | os.PathLike[str]]) -> list[Finding]:
    """
    Find the unreferenced code in a Python project.

    Args:
        paths: The files and directories that make up the project.

    Returns:
        The findings, one for each definition that nothing references, sorted by file and line.
    """
    # References are not tracked yet, so every definition is reported.
    return [finding for path in discover(paths) for finding in find_definitions(path)]
