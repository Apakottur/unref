import ast
import os
from collections.abc import Iterable
from fnmatch import fnmatchcase
from pathlib import Path

from unref._definitions import find_definitions
from unref._discover import discover, is_under
from unref._models import Finding, Override
from unref._references import find_references


def find_unreferenced(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_names: Iterable[str],
    used_decorators: Iterable[str],
    exclude: Iterable[str | os.PathLike[str]],
    overrides: Iterable[Override],
) -> list[Finding]:
    """
    Find the definitions in a Python project that nothing references.

    Args:
        paths: The files and directories that make up the project.
        used_names: The names, as patterns, of the definitions that count as used.
        used_decorators: The decorators that make the definitions they decorate count as used.
        exclude: The files and directories that are not part of the project.
        overrides: The options that apply only to some of the files of the project.

    Returns:
        A finding for each unreferenced definition, sorted by file and line.
    """
    modules = [(path, ast.parse(path.read_bytes(), filename=str(path))) for path in discover(paths, exclude=exclude)]

    # References are matched by name only: a name that is referred to anywhere in the project keeps
    # every definition of that name alive.
    references: set[str] = set()
    for _, module in modules:
        references |= find_references(module)

    project_names = list(used_names)
    project_decorators = set(used_decorators)
    resolved_overrides = [([Path(path).resolve() for path in override.paths], override) for override in overrides]

    findings: list[Finding] = []
    for path, module in modules:
        # The options of a file are those of the project, plus those of every override that covers it.
        file_names = list(project_names)
        file_decorators = set(project_decorators)
        for roots, override in resolved_overrides:
            if is_under(path, roots):
                file_names += override.used_names
                file_decorators |= set(override.used_decorators)

        findings += [
            Finding(
                path=definition.path,
                line=definition.line,
                kind=f"unused {definition.kind}",
                name=definition.name,
            )
            for definition in find_definitions(path, module)
            if definition.name not in references
            and not _is_dunder(definition.name)
            and not any(fnmatchcase(definition.name, pattern) for pattern in file_names)
            and file_decorators.isdisjoint(definition.decorators)
        ]
    return findings


def _is_dunder(name: str) -> bool:
    """
    Check whether a name is a dunder, such as `__init__` or `__all__`.

    Python and tools refer to dunders implicitly, so they always count as referenced.
    """
    return len(name) > 4 and name.startswith("__") and name.endswith("__")
