import ast
import os
from collections.abc import Iterable
from fnmatch import fnmatchcase
from pathlib import Path

from unref._classes import ClassHierarchy
from unref._definitions import find_definitions
from unref._discover import discover, is_under
from unref._models import Finding, Override
from unref._modules import module_name
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
    files = discover(paths, exclude=exclude)
    trees = {path: ast.parse(path.read_bytes(), filename=str(path)) for path in files}
    references = {path: find_references(tree) for path, tree in trees.items()}
    names = {path: module_name(path) for path in files}

    files_by_name: dict[str, list[Path]] = {}
    for path, name in names.items():
        files_by_name.setdefault(name.name, []).append(path)

    # A plain name refers to the module that it is read in, and an imported name to the module that it
    # is imported from.
    module_references = {path: set(references[path].names) for path in files}
    star_imported: set[Path] = set()
    for path in files:
        for source, level, name in references[path].imports:
            for imported in files_by_name.get(names[path].resolve(source, level) or "", []):
                if name == "*":
                    star_imported.add(imported)
                else:
                    module_references[imported].add(name)

    # An attribute is matched by name only: a name that is read as an attribute anywhere in the project
    # keeps every definition of that name alive.
    attribute_references: set[str] = set()
    for path in files:
        attribute_references |= references[path].attributes

    definitions = {path: find_definitions(path, trees[path]) for path in files}
    hierarchy = ClassHierarchy(
        names=names,
        files_by_name=files_by_name,
        bindings={path: references[path].bindings for path in files},
        classes={
            path: {
                definition.qualified_name: definition.bases
                for definition in definitions[path]
                if definition.kind == "class"
            }
            for path in files
        },
    )

    project_names = list(used_names)
    project_decorators = set(used_decorators)
    resolved_overrides = [([Path(path).resolve() for path in override.paths], override) for override in overrides]

    findings: list[Finding] = []
    for path in files:
        # An override with paths applies to the files under them.
        file_overrides = [override for roots, override in resolved_overrides if not roots or is_under(path, roots)]

        # A star import imports the names in `__all__`, which are references of the module already,
        # or every public name if there is no `__all__`.
        all_public = path in star_imported and not references[path].has_all

        for definition in definitions[path]:
            # The options of a definition are those of the project, plus those of every override that
            # covers it.
            names_of_definition = list(project_names)
            decorators_of_definition = set(project_decorators)
            for override in file_overrides:
                # An override with base classes applies to what is defined in a class that inherits
                # from one of them.
                if override.base_classes and (
                    definition.owner is None
                    or hierarchy.ancestors(path, definition.owner).isdisjoint(override.base_classes)
                ):
                    continue
                names_of_definition += override.used_names
                decorators_of_definition |= set(override.used_decorators)

            if (
                definition.name not in module_references[path]
                and definition.name not in attribute_references
                and not (all_public and not definition.name.startswith("_"))
                and not _is_dunder(definition.name)
                and not any(fnmatchcase(definition.name, pattern) for pattern in names_of_definition)
                and decorators_of_definition.isdisjoint(definition.decorators)
            ):
                findings.append(
                    Finding(
                        path=definition.path,
                        line=definition.line,
                        kind=f"unused {definition.kind}",
                        name=definition.name,
                    )
                )
    return findings


def _is_dunder(name: str) -> bool:
    """
    Check whether a name is a dunder, such as `__init__` or `__all__`.

    Python and tools refer to dunders implicitly, so they always count as referenced.
    """
    return len(name) > 4 and name.startswith("__") and name.endswith("__")
