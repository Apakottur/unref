import ast
from pathlib import Path

from unref._definitions import find_definitions
from unref._models import Finding
from unref._references import find_references


def find_unreferenced(files: list[Path]) -> list[Finding]:
    """
    Find the definitions in the given Python files that nothing references.

    Args:
        files: The Python files of the project.

    Returns:
        A finding for each unreferenced definition, in the order of the files.
    """
    modules = [(path, ast.parse(path.read_bytes(), filename=str(path))) for path in files]

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
