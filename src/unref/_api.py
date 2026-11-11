import ast
import os
from collections.abc import Iterable

from unref._definitions import find_definitions
from unref._discover import discover
from unref._models import Finding
from unref._references import find_references


def check(paths: Iterable[str | os.PathLike[str]]) -> list[Finding]:
    """
    Find the unreferenced code in a Python project.

    Args:
        paths: The files and directories that make up the project.

    Returns:
        The findings, one for each definition that nothing references, sorted by file and line.
    """
    modules = [(path, ast.parse(path.read_bytes(), filename=str(path))) for path in discover(paths)]

    # References are matched by name only: a name that is referred to anywhere in the project keeps
    # every definition of that name alive.
    references: set[str] = set()
    for _, module in modules:
        references |= find_references(module)

    return [
        finding
        for path, module in modules
        for finding in find_definitions(path, module)
        if finding.name not in references
    ]
