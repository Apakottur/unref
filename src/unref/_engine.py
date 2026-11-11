import ast
import os
from collections.abc import Iterable

from unref._definitions import find_definitions
from unref._discover import discover
from unref._models import Finding
from unref._references import find_references


def find_unreferenced(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_decorators: Iterable[str],
) -> list[Finding]:
    """
    Find the definitions in a Python project that nothing references.

    Args:
        paths: The files and directories that make up the project.
        used_decorators: The decorators that make the definitions they decorate count as used.

    Returns:
        A finding for each unreferenced definition, sorted by file and line.
    """
    modules = [(path, ast.parse(path.read_bytes(), filename=str(path))) for path in discover(paths)]

    # References are matched by name only: a name that is referred to anywhere in the project keeps
    # every definition of that name alive.
    references: set[str] = set()
    for _, module in modules:
        references |= find_references(module)

    used_decorator_names = set(used_decorators)

    return [
        Finding(
            path=definition.path,
            line=definition.line,
            kind=f"unused {definition.kind}",
            name=definition.name,
        )
        for path, module in modules
        for definition in find_definitions(path, module)
        if definition.name not in references
        and not _is_dunder(definition.name)
        and used_decorator_names.isdisjoint(definition.decorators)
    ]


def _is_dunder(name: str) -> bool:
    """
    Check whether a name is a dunder, such as `__init__` or `__all__`.

    Python and tools refer to dunders implicitly, so they always count as referenced.
    """
    return len(name) > 4 and name.startswith("__") and name.endswith("__")
