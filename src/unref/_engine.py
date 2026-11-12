import os
import sys
import sysconfig
from collections import defaultdict
from collections.abc import Iterable
from dataclasses import dataclass, field
from fnmatch import fnmatchcase
from pathlib import Path

from unref._classes import ClassHierarchy
from unref._definitions import Definition
from unref._discover import discover, is_under
from unref._external import ExternalModules, environment_paths, star_imports
from unref._files import FileFacts, analyze_files
from unref._models import Finding, Override
from unref._modules import import_root, module_name
from unref._references import Owner


def find_unused(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_names: Iterable[str],
    used_decorators: Iterable[str],
    exclude: Iterable[str | os.PathLike[str]],
    overrides: Iterable[Override],
    jobs: int | None,
    python: str | os.PathLike[str] | None = None,
) -> list[Finding]:
    """
    Find the unused definitions of a Python project, which are those that nothing references.

    Args:
        paths: The files and directories that make up the project.
        used_names: The names, as patterns, of the definitions that count as used.
        used_decorators: The decorators that make the definitions they decorate count as used.
        exclude: The files and directories that are not part of the project.
        overrides: The options that apply only to some of the files of the project.
        jobs: The number of processes that analyze the files, or None for one for each CPU.
        python: The Python environment or interpreter of the project, or None for the environment that the
            check runs in.

    Returns:
        A finding for each unused definition, and for each file that does not parse, sorted by file and
        line.
    """
    discovered = discover(paths, exclude=exclude)

    # A file that does not parse is a finding itself, and is left out of the project.
    facts: dict[Path, FileFacts] = {}
    syntax_errors: dict[Path, Finding] = {}
    for path, result in analyze_files(discovered, jobs=jobs).items():
        if isinstance(result, Finding):
            syntax_errors[path] = result
        else:
            facts[path] = result
    files = [path for path in discovered if path in facts]

    references = {path: facts[path].references for path in files}
    definitions = {path: facts[path].definitions for path in files}
    names = {path: module_name(path) for path in files}

    files_by_name: dict[str, list[Path]] = {}
    for path, name in names.items():
        files_by_name.setdefault(name.name, []).append(path)

    # What the code of each owner uses. A plain name refers to the module that it is read in, an
    # imported name to the module that it is imported from, and an attribute of an object is matched by
    # name only.
    uses: dict[tuple[Path, Owner], _Uses] = defaultdict(_Uses)
    for path in files:
        for owner, name in references[path].names:
            uses[path, owner].names.add((path, name))

        for owner, source, level, name in references[path].imports:
            for imported in files_by_name.get(names[path].resolve(source, level) or "", []):
                if name == "*":
                    uses[path, owner].stars.add(imported)
                else:
                    uses[path, owner].names.add((imported, name))

        for owner, name in references[path].attributes:
            uses[path, owner].attributes.add(name)

        # An attribute of an imported name is resolved: `package.module.name` refers to `name` in the
        # module `package.module`.
        for owner, (source, level, name), attributes in references[path].imported_attributes:
            module = names[path].resolve(source, level)
            if module is None:
                uses[path, owner].attributes.update(attributes)
                continue
            origin = [*module.split("."), *filter(None, [name])]
            parts = [*origin, *attributes]
            # What follows the last name that a module of the project holds is read from an object.
            first_of_object = len(origin)
            for index in range(1, len(parts)):
                for imported in files_by_name.get(".".join(parts[:index]), []):
                    uses[path, owner].names.add((imported, parts[index]))
                    first_of_object = max(first_of_object, index + 1)
            uses[path, owner].attributes.update(parts[first_of_object:])

    # A module from outside the project is looked for where the modules of the project are imported
    # from first, as an excluded module is, and then where the check imports from. The site-packages
    # of the environment of the project, if it is given, replace those of the check.
    imported_from = list(map(Path, sys.path))
    if python is not None:
        own_site_packages = {Path(sysconfig.get_path(name)) for name in ("purelib", "platlib")}
        imported_from = [
            *environment_paths(Path(python)),
            *(path for path in imported_from if path not in own_site_packages),
        ]
    search_path = list(dict.fromkeys([*(import_root(path, names[path]) for path in files), *imported_from]))
    hierarchy = ClassHierarchy(
        names=names,
        files_by_name=files_by_name,
        bindings={path: references[path].bindings for path in files},
        stars={path: star_imports(references[path]) for path in files},
        lazy={path: facts[path].lazy for path in files},
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
        external=ExternalModules(search_path),
    )

    project_names = list(used_names)
    project_decorators = set(used_decorators)
    resolved_overrides = [([Path(path).resolve() for path in override.paths], override) for override in overrides]

    # The definitions that count as used whatever references them: by the options, or as dunders.
    kept: set[Definition] = set()
    ignored: set[Definition] = set()
    ignored_lines: dict[Path, set[int]] = {}
    for path in files:
        # An override with paths applies to the files under them.
        file_overrides = [override for roots, override in resolved_overrides if not roots or is_under(path, roots)]
        ignored_lines[path] = facts[path].ignored_lines

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
                definition.name == "_"
                or _is_dunder(definition.name)
                # A member that a base class outside the project defines is used by its code.
                or (
                    definition.owner is not None
                    and definition.name in hierarchy.external_members(path, definition.owner)
                )
                or any(fnmatchcase(definition.name, pattern) for pattern in names_of_definition)
                or not decorators_of_definition.isdisjoint(definition.decorators)
            ):
                kept.add(definition)
            elif definition.line in ignored_lines[path]:
                ignored.add(definition)

    graph = _Graph(
        files=files,
        definitions=[definition for path in files for definition in definitions[path]],
        uses=uses,
        has_all={path for path in files if references[path].has_all},
    )

    # An ignore comment is needed for a definition that is unused without it. Such a definition is
    # then used, and so is what it uses.
    needed = ignored - graph.used(kept)
    used = graph.used(kept | needed)

    findings: list[Finding] = []
    for path in discovered:
        if path in syntax_errors:
            findings.append(syntax_errors[path])
            continue

        file_findings = [
            Finding(
                path=definition.path,
                line=definition.line,
                kind=f"unused {definition.kind}",
                name=definition.name,
            )
            for definition in definitions[path]
            if definition not in used
        ]

        # An ignore comment that ignores nothing is a finding itself.
        needed_lines = {definition.line for definition in needed if definition.path == path}
        file_findings.extend(
            Finding(path=path, line=line, kind="unused comment", name="unref: ignore")
            for line in ignored_lines[path] - needed_lines
        )
        findings += sorted(file_findings, key=lambda finding: finding.line)
    return findings


@dataclass
class _Uses:
    """
    What the code of an owner uses.

    Attributes:
        names: The names of the modules of the project that the code uses, each with its module.
        attributes: The names that the code reads as attributes of an object.
        stars: The modules that the code star imports.
    """

    names: set[tuple[Path, str]] = field(default_factory=set)
    attributes: set[str] = field(default_factory=set)
    stars: set[Path] = field(default_factory=set)


class _Graph:
    """
    The definitions of a project and what the code of each owner uses.

    The code of a module runs when it is imported, and the code of a function only when it is called.
    So a function that is unused uses nothing.
    """

    def __init__(
        self,
        *,
        files: list[Path],
        definitions: list[Definition],
        uses: dict[tuple[Path, Owner], _Uses],
        has_all: set[Path],
    ) -> None:
        """
        Args:
            files: The files of the project.
            definitions: The definitions of the project.
            uses: What the code of each owner uses.
            has_all: The modules that define `__all__`.
        """
        self._files = files
        self._uses = uses
        self._by_name: dict[tuple[Path, str], list[Definition]] = defaultdict(list)
        self._by_attribute: dict[str, list[Definition]] = defaultdict(list)
        self._public: dict[Path, list[Definition]] = defaultdict(list)
        for definition in definitions:
            self._by_name[definition.path, definition.name].append(definition)
            # Only a method or an attribute is read as an attribute of an object.
            if definition.owner is not None:
                self._by_attribute[definition.name].append(definition)
            # A star import imports the names in `__all__`, which are references of the module
            # already, or every public name if there is no `__all__`.
            if definition.path not in has_all and not definition.name.startswith("_"):
                self._public[definition.path].append(definition)

    def used(self, roots: set[Definition]) -> set[Definition]:
        """
        Get the definitions that are used.

        Args:
            roots: The definitions that are used whatever references them.

        Returns:
            The roots, and the definitions that the code of the modules uses, directly or through the
            functions that it uses.
        """
        used: set[Definition] = set()
        pending: list[tuple[Path, Owner]] = [(path, None) for path in self._files]

        def use(definitions: Iterable[Definition]) -> None:
            for definition in definitions:
                if definition not in used:
                    used.add(definition)
                    if definition.kind in ("function", "method"):
                        pending.append((definition.path, definition.line))

        use(roots)
        seen_owners: set[tuple[Path, Owner]] = set()
        seen_names: set[tuple[Path, str]] = set()
        seen_attributes: set[str] = set()
        seen_stars: set[Path] = set()
        while pending:
            owner = pending.pop()
            if owner in seen_owners or owner not in self._uses:
                continue
            seen_owners.add(owner)
            uses = self._uses[owner]

            for name in uses.names - seen_names:
                seen_names.add(name)
                use(self._by_name.get(name, []))
            for attribute in uses.attributes - seen_attributes:
                seen_attributes.add(attribute)
                use(self._by_attribute.get(attribute, []))
            for star in uses.stars - seen_stars:
                seen_stars.add(star)
                use(self._public.get(star, []))
        return used


def _is_dunder(name: str) -> bool:
    """
    Check whether a name is a dunder, such as `__init__` or `__all__`.

    Python and tools refer to dunders implicitly, so they always count as used.
    """
    return len(name) > 4 and name.startswith("__") and name.endswith("__")
