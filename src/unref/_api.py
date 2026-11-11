import os
from collections.abc import Iterable

from unref._models import Finding


def check(paths: Iterable[str | os.PathLike[str]]) -> list[Finding]:  # noqa: ARG001 (nothing is analyzed yet)
    """
    Find the unreferenced code in a Python project.

    Args:
        paths: The files and directories that make up the project.

    Returns:
        The findings, one for each definition that nothing references.
    """
    return []
