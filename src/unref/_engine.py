import ast
import os
from collections.abc import Iterable
from fnmatch import fnmatchcase
from pathlib import Path

from unref._classes import ClassHierarchy
from unref._definitions import find_definitions
from unref._discover import discover, is_under
from unref._ignores import find_ignored_lines
from unref._models import Finding, Override
from unref._modules import module_name
from unref._references import find_references


def find_unused(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_names: Iterable[str],
    used_decorators: Iterable[str],
    exclude: Iterable[str | os.PathLike[str]],
    overrides: Iterable[Override],
) -> list[Finding]:
    """
    Find the unused definitions of a Python project, which are those that nothing references.

    Args:
        paths: The files and directories that make up the project.
        used_names: The names, as patterns, of the definitions that count as used.
        used_decorators: The decorators that make the definitions they decorate count as used.
        exclude: The files and directories that are not part of the project.
        overrides: The options that apply only to some of the files of the project.

    Returns:
        A finding for each unused definition, sorted by file and line.
    """
    files = discover(paths, exclude=exclude)
    sources = {path: path.read_bytes() for path in files}
    trees = {path: ast.parse(source, filename=str(path)) for path, source in sources.items()}
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

    # An attribute of an object is matched by name only: a name that is read as an attribute anywhere
    # in the project keeps every method and attribute of that name alive.
    attribute_references: set[str] = set()
    for path in files:
        attribute_references |= references[path].attributes

        # An attribute of an imported name is resolved: `package.module.name` refers to `name` in the
        # module `package.module`.
        for (source, level, name), attributes in references[path].imported_attributes:
            module = names[path].resolve(source, level)
            if module is None:
                attribute_references.update(attributes)
                continue
            origin = [*module.split("."), *filter(None, [name])]
            parts = [*origin, *attributes]
            # What follows the last name that a module of the project holds is read from an object.
            first_of_object = len(origin)
            for index in range(1, len(parts)):
                for imported in files_by_name.get(".".join(parts[:index]), []):
                    module_references[imported].add(parts[index])
                    first_of_object = max(first_of_object, index + 1)
            attribute_references.update(parts[first_of_object:])

    definitions = {path: find_definitions(path, trees[path]) for path in files}
    hierarchy = ClassHierarchy(
        names=names,
        files_by_name=files_by_name,
        bindings={path: references[path].bindings for path in files},
        classes={
            path: {
                definition.qualified_name: definition.bases
                for definition in definitions[path]
                if definition.kind in ("class", "function")
            }
            for path in files
        },
        decorators={
            path: {
                definition.qualified_name: definition.decorators
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

        ignored_lines = find_ignored_lines(sources[path])
        unused_ignored_lines = set(ignored_lines)
        file_findings: list[Finding] = []

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
                # An override with class decorators applies to what is defined in a class that has
                # one of them.
                if override.class_decorators and (
                    definition.owner is None
                    or hierarchy.decorators(path, definition.owner).isdisjoint(override.class_decorators)
                ):
                    continue
                names_of_definition += override.used_names
                decorators_of_definition |= set(override.used_decorators)

            if (
                definition.name not in module_references[path]
                and not (definition.owner is not None and definition.name in attribute_references)
                and not (all_public and not definition.name.startswith("_"))
                and not _is_dunder(definition.name)
                and not any(fnmatchcase(definition.name, pattern) for pattern in names_of_definition)
                and decorators_of_definition.isdisjoint(definition.decorators)
            ):
                if definition.line in ignored_lines:
                    unused_ignored_lines.discard(definition.line)
                    continue
                file_findings.append(
                    Finding(
                        path=definition.path,
                        line=definition.line,
                        kind=f"unused {definition.kind}",
                        name=definition.name,
                    )
                )

        # An ignore comment that ignores nothing is a finding itself.
        file_findings.extend(
            Finding(path=path, line=line, kind="unused comment", name="unref: ignore") for line in unused_ignored_lines
        )
        findings += sorted(file_findings, key=lambda finding: finding.line)
    return findings


def _is_dunder(name: str) -> bool:
    """
    Check whether a name is a dunder, such as `__init__` or `__all__`.

    Python and tools refer to dunders implicitly, so they always count as used.
    """
    return len(name) > 4 and name.startswith("__") and name.endswith("__")
