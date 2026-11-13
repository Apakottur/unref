import os
import re
import sys
import sysconfig
from collections import defaultdict
from collections.abc import Collection, Iterable, Iterator, Mapping
from dataclasses import dataclass, field
from fnmatch import fnmatchcase
from pathlib import Path

from unref._classes import ClassHierarchy
from unref._definitions import Definition
from unref._discover import discover, is_under
from unref._external import ExternalModules, environment_paths
from unref._files import FileFacts, analyze_files
from unref._frameworks import FrameworkRules
from unref._ignores import apply_ignore_comments, ignored_codes
from unref._models import FRAMEWORKS, Finding, Framework, Override
from unref._modules import ModuleName, import_root, module_name
from unref._references import Binding, Owner
from unref._symbols import ModuleSymbols
from unref._transitive import transitive

# The object reference of an entry point, as the entry points specification of the Python Packaging
# User Guide gives it.
_OBJECT_REFERENCE = re.compile(r"(?P<module>[\w.]+)\s*(:\s*(?P<attributes>[\w.]+)\s*)?(\[.*\]\s*)?")
# `TypedDict`, as `typing` and `typing_extensions` give it out.
_TYPED_DICTS = frozenset({"typing.TypedDict", "typing_extensions.TypedDict"})
# `Enum`, which `IntEnum`, `StrEnum`, `Flag` and the other enums of `enum` inherit from.
_ENUMS = frozenset({"enum.Enum"})
# `ast.NodeVisitor`, whose `visit` calls the method named `visit_` and the class of the node, and
# `ast.NodeTransformer`, which inherits from it.
_NODE_VISITORS = frozenset({"ast.NodeVisitor", "ast.NodeTransformer"})
_NODE_VISITOR_PREFIX = "visit_"


@dataclass
class _Uses:
    """
    What the code of an owner uses.

    Attributes:
        names: The names of the modules of the project that the code uses, each with its module.
        attributes: The names that the code reads as attributes of an object.
        keys: The string keys that the code reads from a mapping.
        objects: The objects of the modules of the project that the code reads, each with its module
            and its name in the module followed by the attributes that are read from it, in order.
        stars: The modules that the code star imports.
    """

    names: set[tuple[Path, str]] = field(default_factory=set)
    attributes: set[str] = field(default_factory=set)
    keys: set[str] = field(default_factory=set)
    objects: set[tuple[Path, tuple[str, ...]]] = field(default_factory=set)
    stars: set[Path] = field(default_factory=set)


def find_unused(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_names: Iterable[str],
    used_decorators: Iterable[str],
    exclude: Iterable[str],
    overrides: Iterable[Override],
    jobs: int | None,
    python: str | os.PathLike[str] | None,
    dummy_variable_rgx: str,
    entry_points: Iterable[str],
    frameworks: Mapping[Framework, bool],
) -> list[Finding]:
    """
    Find the unused definitions of a Python project, which are those that nothing references.

    Args:
        paths: The files and directories that make up the project.
        used_names: The names, as patterns, of the definitions that count as used.
        used_decorators: The decorators that make the definitions they decorate count as used.
        exclude: The patterns of the files and directories that are not part of the project.
        overrides: The options that apply only to some of the files of the project.
        jobs: The number of processes that analyze the files, or None for one for each CPU.
        python: The Python environment or interpreter of the project, or None for the environment that the
            check runs in.
        dummy_variable_rgx: The pattern of the names of the variables that are not used on purpose.
        entry_points: The object references of the entry points of the project.
        frameworks: Whether the rules of each framework apply, over the defaults.

    Returns:
        A finding for each unused definition, for each problem with an ignore comment, and for each file
        that does not parse, sorted by file, line and column.

    Raises:
        ValueError: If a framework is not a known one, or if `jobs` is less than 1.
    """
    paths = list(paths)
    # A caller without type checks can give a name that is not a framework.
    unknown = sorted(str(name) for name in frameworks if name not in FRAMEWORKS)
    if unknown:
        raise ValueError(
            f"Unknown framework {unknown[0]!r}, expected one of {', '.join(repr(str(name)) for name in FRAMEWORKS)}"
        )
    enabled = {framework for framework, default in FRAMEWORKS.items() if frameworks.get(framework, default)}

    discovered = discover(paths, exclude=exclude)
    facts, syntax_errors = _analyze(discovered, jobs=jobs)
    files = [path for path in discovered if path in facts]

    # Only a directory can have a namespace package under it. A hook of prek or pre-commit gives every
    # file of the project, and checking each directory against each of them takes quadratic time.
    roots = [root for root in (Path(path).resolve() for path in paths) if root.is_dir()]
    names = {path: module_name(path, roots) for path in files}
    files_by_name: dict[str, list[Path]] = {}
    for path, name in names.items():
        files_by_name.setdefault(name.name, []).append(path)

    uses = _uses(files, facts=facts, names=names, files_by_name=files_by_name)
    modules = {
        path: ModuleSymbols.of(path, names[path], facts[path].definitions, facts[path].references, facts[path].lazy)
        for path in files
    }
    hierarchy = _hierarchy(modules, files_by_name=files_by_name, python=python)
    fields, enums = _subclass_attributes(files, facts=facts, hierarchy=hierarchy, bases=(_TYPED_DICTS, _ENUMS))
    kept, ignored = _roots(
        files,
        facts=facts,
        hierarchy=hierarchy,
        used_names=list(used_names),
        used_decorators=set(used_decorators),
        overrides=overrides,
        dummy_variable_rgx=dummy_variable_rgx,
        entry_objects=_entry_objects(entry_points, files_by_name=files_by_name),
        rules=FrameworkRules(enabled, modules=modules, hierarchy=hierarchy),
    )

    graph = _Graph(
        files=files,
        definitions=[definition for path in files for definition in facts[path].definitions],
        uses=uses,
        fields={definition for members in fields.values() for definition in members},
        enums=enums,
        has_all={path for path in files if facts[path].references.has_all},
    )

    # An ignore comment is needed for a definition that is unused without it. Such a definition is
    # then used, and so is what it uses.
    needed = ignored - graph.use(kept)
    used = graph.use(needed)
    unused = [definition for path in files for definition in facts[path].definitions if definition not in used]

    return _findings(
        discovered,
        facts=facts,
        syntax_errors=syntax_errors,
        used=used,
        needed=needed,
        transitive=transitive(_unused_reached(unused, graph=graph)),
    )


def _analyze(discovered: list[Path], *, jobs: int | None) -> tuple[dict[Path, FileFacts], dict[Path, Finding]]:
    """
    Analyze the files of a project.

    A file that does not parse is a finding itself, and is left out of the project.

    Args:
        discovered: The files of the project.
        jobs: The number of processes that analyze the files, or None for one for each CPU.

    Returns:
        What each file that parses defines and refers to, and the finding of each file that does not.
    """
    facts: dict[Path, FileFacts] = {}
    syntax_errors: dict[Path, Finding] = {}
    for path, result in analyze_files(discovered, jobs=jobs).items():
        if isinstance(result, Finding):
            syntax_errors[path] = result
        else:
            facts[path] = result
    return facts, syntax_errors


def _uses(
    files: list[Path],
    *,
    facts: dict[Path, FileFacts],
    names: dict[Path, ModuleName],
    files_by_name: dict[str, list[Path]],
) -> dict[tuple[Path, Owner], _Uses]:
    """
    Find what the code of each owner uses.

    A plain name refers to the module that it is read in, an imported name to the module that it is
    imported from, and an attribute of an object is matched by name only.

    Args:
        files: The files of the project.
        facts: What each file defines and refers to.
        names: The module name of each file.
        files_by_name: The files of each module name.

    Returns:
        What the code of each owner uses.
    """
    uses: dict[tuple[Path, Owner], _Uses] = defaultdict(_Uses)
    for path in files:
        references = facts[path].references
        for owner, root, attributes, reads_object in references.reads:
            owner_uses = uses[path, owner]
            owner_uses.names.add((path, root))
            owner_uses.attributes.update(attributes)
            if reads_object:
                owner_uses.objects.add((path, (root, *attributes)))

        for owner, binding, attributes, reads_object in references.imported_reads:
            owner_uses = uses[path, owner]
            resolved = _resolve(names[path], binding, attributes, files_by_name)
            if resolved is None:
                owner_uses.attributes.update(attributes)
                continue
            if reads_object:
                owner_uses.objects.update(
                    _imported_objects(resolved, facts=facts, names=names, files_by_name=files_by_name)
                )
            if not attributes:
                continue
            # An attribute of an imported name is resolved: `package.module.name` is `name` in `package.module`.
            parts, starts = resolved
            for index, modules in starts:
                owner_uses.names.update((module, parts[index]) for module in modules)
            # What follows the last name that a module of the project holds is read from an object.
            first_of_object = max([len(parts) - len(attributes), *(index + 1 for index, _ in starts)])
            owner_uses.attributes.update(parts[first_of_object:])

        for owner, source, level, name in references.imports:
            for imported in files_by_name.get(names[path].resolve(source, level) or "", []):
                if name == "*":
                    uses[path, owner].stars.add(imported)
                else:
                    uses[path, owner].names.add((imported, name))

        for owner, name in references.attributes:
            uses[path, owner].attributes.add(name)
        for owner, name in references.keys:
            uses[path, owner].keys.add(name)
    return uses


def _resolve(
    module: ModuleName, binding: Binding, attributes: tuple[str, ...], files_by_name: dict[str, list[Path]]
) -> tuple[list[str], list[tuple[int, list[Path]]]] | None:
    """
    Find the dotted name that an imported name, and the attributes read from it, stand for.

    Args:
        module: The module that imports the name.
        binding: What the imported name stands for.
        attributes: The attributes that are read from the imported name, in order.
        files_by_name: The files of each module name.

    Returns:
        The parts of the name, and each shorter start of it that is a module of the project, shortest
        first, as its number of parts with its files. None if it is imported from beyond the root.
    """
    source, level, name = binding
    resolved = module.resolve(source, level)
    if resolved is None:
        return None
    parts = [*resolved.split("."), *filter(None, [name]), *attributes]
    starts: list[tuple[int, list[Path]]] = []
    for index in range(1, len(parts)):
        files = files_by_name.get(".".join(parts[:index]))
        if files is not None:
            starts.append((index, files))
    return parts, starts


def _imported_objects(
    resolved: tuple[list[str], list[tuple[int, list[Path]]]],
    *,
    facts: dict[Path, FileFacts],
    names: dict[Path, ModuleName],
    files_by_name: dict[str, list[Path]],
) -> set[tuple[Path, tuple[str, ...]]]:
    """
    Find the objects of the modules of the project that an imported name, and the attributes read from
    it, stand for, through the modules that import the name in turn.

    Args:
        resolved: The dotted name of the imported name and its attributes, as `_resolve` finds it.
        facts: What each file defines and refers to.
        names: The module name of each file.
        files_by_name: The files of each module name.

    Returns:
        Each object with its module, and its name in the module followed by the attributes.
    """
    found: set[tuple[Path, tuple[str, ...]]] = set()
    # Each binding that is followed is followed once, which ends a cycle of modules that import the
    # names of each other.
    followed: set[tuple[Path, str]] = set()
    pending = [resolved]
    while pending:
        parts, starts = pending.pop()
        if not starts:
            continue
        for index, modules in starts:
            found.update((module, tuple(parts[index:])) for module in modules)
        # A module that imports the name in turn, such as a package that re-exports it. Only the longest
        # module is followed, as `package.module.name` reads `name` from `package.module`.
        index, modules = starts[-1]
        for module in modules:
            module_binding = facts[module].references.bindings.get(parts[index])
            if module_binding is not None and (module, parts[index]) not in followed:
                followed.add((module, parts[index]))
                followed_name = _resolve(names[module], module_binding, tuple(parts[index + 1 :]), files_by_name)
                if followed_name is not None:
                    pending.append(followed_name)
    return found


def _hierarchy(
    modules: dict[Path, ModuleSymbols],
    *,
    files_by_name: dict[str, list[Path]],
    python: str | os.PathLike[str] | None,
) -> ClassHierarchy:
    """
    Build the hierarchy of the classes of a project.

    A module from outside the project is looked for where the modules of the project are imported from
    first, as an excluded module is, and then where the check imports from. The site-packages of the
    environment of the project, if it is given, replace those of the check.

    Args:
        modules: The symbols of each module of the project.
        files_by_name: The files of each module name.
        python: The Python environment or interpreter of the project, or None for the environment that the
            check runs in.

    Returns:
        The hierarchy of the classes of the project.
    """
    imported_from = list(map(Path, sys.path))
    if python is not None:
        own_site_packages = {Path(sysconfig.get_path(name)) for name in ("purelib", "platlib")}
        imported_from = [
            *environment_paths(Path(python)),
            *(path for path in imported_from if path not in own_site_packages),
        ]
    search_path = list(
        dict.fromkeys([*(import_root(path, module.name) for path, module in modules.items()), *imported_from])
    )
    return ClassHierarchy(modules, files_by_name=files_by_name, external=ExternalModules(search_path))


def _subclass_attributes(
    files: list[Path], *, facts: dict[Path, FileFacts], hierarchy: ClassHierarchy, bases: Iterable[frozenset[str]]
) -> list[dict[tuple[Path, str], list[Definition]]]:
    """
    Find the attributes of the classes of a project that inherit from given base classes, such as the
    fields of the TypedDicts, which are read as keys rather than as attributes, or the members of the
    enums.

    Args:
        files: The files of the project.
        facts: What each file defines and refers to.
        hierarchy: The hierarchy of the classes of the project.
        bases: The sets of dotted names of base classes.

    Returns:
        For each set of base classes, the attributes of each class that inherits from one of them,
        directly or not, by its file and its name in the module.
    """
    attributes: dict[tuple[Path, str], list[Definition]] = defaultdict(list)
    for path in files:
        for definition in facts[path].definitions:
            if definition.kind == "attribute" and definition.owner is not None:
                attributes[path, definition.owner].append(definition)
    return [
        {key: members for key, members in attributes.items() if not hierarchy.ancestors(*key).isdisjoint(names)}
        for names in bases
    ]


def _roots(
    files: list[Path],
    *,
    facts: dict[Path, FileFacts],
    hierarchy: ClassHierarchy,
    used_names: list[str],
    used_decorators: set[str],
    overrides: Iterable[Override],
    dummy_variable_rgx: str,
    entry_objects: dict[Path, set[str]],
    rules: FrameworkRules,
) -> tuple[set[Definition], set[Definition]]:
    """
    Find the definitions that count as used whatever references them, and those that are ignored.

    Args:
        files: The files of the project.
        facts: What each file defines and refers to.
        hierarchy: The hierarchy of the classes of the project.
        used_names: The names, as patterns, of the definitions that count as used.
        used_decorators: The decorators that make the definitions they decorate count as used.
        overrides: The options that apply only to some of the files of the project.
        dummy_variable_rgx: The pattern of the names of the variables that are not used on purpose.
        entry_objects: The qualified names of the objects that the entry points name, in each file.
        rules: The rules of the test frameworks of the project.

    Returns:
        The definitions that count as used by the options or as dunders, and the other definitions that
        an ignore comment ignores.
    """
    dummy_variable = re.compile(dummy_variable_rgx)
    resolved_overrides = [([Path(path).resolve() for path in override.paths], override) for override in overrides]

    kept: set[Definition] = set()
    ignored: set[Definition] = set()
    for path in files:
        # An override with paths applies to the files under them.
        file_overrides = [override for roots, override in resolved_overrides if not roots or is_under(path, roots)]

        ignored_in_file = ignored_codes(facts[path].ignore_comments)
        for definition in facts[path].definitions:
            if (
                definition.name == "_"
                or _is_dunder(definition.name)
                # A variable that is not used on purpose, where Python needs a name to bind. Ruff
                # searches for the pattern, rather than matching the whole name.
                or (definition.unpacked and dummy_variable.search(definition.name) is not None)
                # A member that a base class outside the project defines is used by its code.
                or (
                    definition.owner is not None
                    and definition.name in hierarchy.external_members(path, definition.owner)
                )
                # A method of a node visitor of `ast`, which its `visit` calls by the name of the node.
                or (
                    definition.kind == "method"
                    and definition.owner is not None
                    and definition.name.startswith(_NODE_VISITOR_PREFIX)
                    and not hierarchy.ancestors(path, definition.owner).isdisjoint(_NODE_VISITORS)
                )
                # The objects that the entry points name.
                or definition.qualified_name in entry_objects.get(path, set())
                # What the test frameworks find and call by themselves.
                or rules.uses(definition, path)
                # The options of a definition are those of the project, plus those of every override
                # that covers it.
                or _matches(definition, names=used_names, decorators=used_decorators)
                or any(
                    _covers(override, path, definition, hierarchy)
                    and _matches(definition, names=override.used_names, decorators=override.used_decorators)
                    for override in file_overrides
                )
            ):
                kept.add(definition)
            elif (definition.line, _code(definition)) in ignored_in_file:
                ignored.add(definition)
    return kept, ignored


def _unused_reached(unused: list[Definition], *, graph: "_Graph") -> dict[Definition, list[Definition]]:
    """
    Find the unused definitions that each unused definition reaches.

    An unused function reaches what its code uses, as it would if it were used. An unused class reaches
    what is defined in it, and an unused function what is defined in it, since deleting them deletes
    those too. An ignored definition is used, so it reaches nothing here.

    Args:
        unused: The unused definitions.
        graph: The definitions of the project and what the code of each owner uses.

    Returns:
        The unused definitions that each unused definition reaches.
    """
    is_unused = set(unused)
    reached = {
        definition: [other for other in graph.reached(definition) if other in is_unused] for definition in unused
    }
    classes = {
        (definition.path, definition.qualified_name): definition for definition in unused if definition.kind == "class"
    }
    # By the owner of their code, which is None for no function.
    functions: dict[tuple[Path, Owner], Definition] = {
        (definition.path, definition.line): definition
        for definition in unused
        if definition.kind in ("function", "method")
    }
    for definition in unused:
        # A definition in a class is in the class first, even if the class is in a function.
        container = (
            classes.get((definition.path, definition.owner))
            if definition.owner is not None
            else functions.get((definition.path, definition.function))
        )
        if container is not None:
            reached[container].append(definition)
    return reached


def _covers(override: Override, path: Path, definition: Definition, hierarchy: ClassHierarchy) -> bool:
    """
    Check whether an override that applies to a file covers a definition of it.

    Args:
        override: The override.
        path: The file of the definition.
        definition: The definition.
        hierarchy: The hierarchy of the classes of the project.

    Returns:
        Whether the definition matches the base classes and the class decorators of the override.
    """
    # An override with base classes applies to what is defined in a class that inherits from one of
    # them.
    if override.base_classes and (
        definition.owner is None or hierarchy.ancestors(path, definition.owner).isdisjoint(override.base_classes)
    ):
        return False
    # An override with class decorators applies to what is defined in a class that has one of them.
    return not override.class_decorators or (
        definition.owner is not None
        and not hierarchy.decorators(path, definition.owner).isdisjoint(override.class_decorators)
    )


def _matches(definition: Definition, *, names: Iterable[str], decorators: Collection[str]) -> bool:
    """
    Check whether options make a definition count as used.

    Args:
        definition: The definition.
        names: The names, as patterns, of the definitions that count as used.
        decorators: The decorators that make the definitions they decorate count as used.

    Returns:
        Whether the name of the definition matches one of the patterns, or it has one of the decorators.
    """
    return any(fnmatchcase(definition.name, pattern) for pattern in names) or any(
        decorator in decorators for decorator in definition.decorators
    )


def _findings(
    discovered: list[Path],
    *,
    facts: dict[Path, FileFacts],
    syntax_errors: dict[Path, Finding],
    used: set[Definition],
    needed: set[Definition],
    transitive: set[Definition],
) -> list[Finding]:
    """
    Make the findings of a project.

    Args:
        discovered: The files of the project.
        facts: What each file that parses defines and refers to.
        syntax_errors: The finding of each file that does not parse.
        used: The definitions that are used.
        needed: The definitions that an ignore comment is needed for.
        transitive: The unused definitions that only other unused definitions reach.

    Returns:
        A finding for each unused definition, for each problem with an ignore comment, and for each file
        that does not parse, sorted by file, line and column.
    """
    findings: list[Finding] = []
    for path in discovered:
        if path in syntax_errors:
            findings.append(syntax_errors[path])
            continue

        file_findings = [
            Finding(
                path=definition.path,
                line=definition.line,
                column=definition.column,
                code=_code(definition),
                name=definition.name,
                transitive=definition in transitive,
            )
            for definition in facts[path].definitions
            if definition not in used
        ]

        # A code of an ignore comment that ignores nothing is a finding itself.
        file_findings += apply_ignore_comments(
            path,
            facts[path].ignore_comments,
            facts[path].ignore_findings,
            ignored=[(definition.line, _code(definition)) for definition in needed if definition.path == path],
        )
        findings += sorted(file_findings, key=lambda finding: (finding.line, finding.column))
    return findings


def _code(definition: Definition) -> str:
    """
    Get the code of the finding of a definition that is unused, such as `unused-function`.
    """
    return f"unused-{definition.kind}"


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
        fields: set[Definition],
        enums: dict[tuple[Path, str], list[Definition]],
        has_all: set[Path],
    ) -> None:
        """
        Args:
            files: The files of the project.
            definitions: The definitions of the project.
            uses: What the code of each owner uses.
            fields: The fields of the TypedDicts of the project.
            enums: The members of each enum of the project, by its file and its name in the module.
            has_all: The modules that define `__all__`.
        """
        self._uses = uses
        self._enums = {key: (members, {member.name for member in members}) for key, members in enums.items()}
        self._by_name: dict[tuple[Path, str], list[Definition]] = defaultdict(list)
        self._by_attribute: dict[str, list[Definition]] = defaultdict(list)
        self._by_key: dict[str, list[Definition]] = defaultdict(list)
        self._public: dict[Path, list[Definition]] = defaultdict(list)
        for definition in definitions:
            self._by_name[definition.path, definition.name].append(definition)
            # A field of a TypedDict is read as a key, and only a method or another attribute is read
            # as an attribute of an object.
            if definition in fields:
                self._by_key[definition.name].append(definition)
            elif definition.owner is not None:
                self._by_attribute[definition.name].append(definition)
            # A star import imports the names in `__all__`, which are references of the module
            # already, or every public name if there is no `__all__`.
            if definition.path not in has_all and not definition.name.startswith("_"):
                self._public[definition.path].append(definition)

        # What is used so far, and what is left to follow. The code of every module runs.
        self._used: set[Definition] = set()
        self._pending: list[tuple[Path, Owner]] = [(path, None) for path in files]
        self._seen_owners: set[tuple[Path, Owner]] = set()
        self._seen_names: set[tuple[Path, str]] = set()
        self._seen_attributes: set[str] = set()
        self._seen_keys: set[str] = set()
        self._seen_objects: set[tuple[Path, tuple[str, ...]]] = set()
        self._seen_stars: set[Path] = set()

    def use(self, roots: Iterable[Definition]) -> set[Definition]:
        """
        Use more definitions, and get all the definitions that are used so far.

        The first call also uses what the code of the modules uses. Each later call goes on from where
        the one before stopped.

        Args:
            roots: The definitions that are used whatever references them.

        Returns:
            The roots of every call, and the definitions that the code of the modules uses, directly or
            through the functions that it uses.
        """
        self._add(roots)
        while self._pending:
            owner = self._pending.pop()
            if owner in self._seen_owners or owner not in self._uses:
                continue
            self._seen_owners.add(owner)
            uses = self._uses[owner]

            for name in uses.names - self._seen_names:
                self._seen_names.add(name)
                self._add(self._by_name.get(name, []))
            for attribute in uses.attributes - self._seen_attributes:
                self._seen_attributes.add(attribute)
                self._add(self._by_attribute.get(attribute, []))
            for key in uses.keys - self._seen_keys:
                self._seen_keys.add(key)
                self._add(self._by_key.get(key, []))
            for read in uses.objects - self._seen_objects:
                self._seen_objects.add(read)
                for members in self._enums_read(*read):
                    self._add(members)
            for star in uses.stars - self._seen_stars:
                self._seen_stars.add(star)
                self._add(self._public.get(star, []))
        return set(self._used)

    def reached(self, definition: Definition) -> set[Definition]:
        """
        Find the definitions that the code of a function uses directly, as if it were used.

        Args:
            definition: The definition, which reaches nothing if it is not a function.

        Returns:
            The definitions that the code of the function uses.
        """
        uses = self._uses.get((definition.path, definition.line))
        if definition.kind not in ("function", "method") or uses is None:
            return set()
        reached: set[Definition] = set()
        for name in uses.names:
            reached.update(self._by_name.get(name, []))
        for attribute in uses.attributes:
            reached.update(self._by_attribute.get(attribute, []))
        for key in uses.keys:
            reached.update(self._by_key.get(key, []))
        for read in uses.objects:
            for members in self._enums_read(*read):
                reached.update(members)
        for star in uses.stars:
            reached.update(self._public.get(star, []))
        return reached

    def _enums_read(self, path: Path, parts: tuple[str, ...]) -> Iterator[list[Definition]]:
        """
        Find the members of the enums that an object read uses as a whole.

        A member read as an attribute, as in `Color.RED`, is used by its name. Any other read of an enum,
        such as `Color(value)`, `list(Color)`, `Color.parse(text)` or an annotation, can reach any
        member, so it uses them all.
        """
        for index in range(1, len(parts) + 1):
            enum = self._enums.get((path, ".".join(parts[:index])))
            if enum is not None and (index == len(parts) or parts[index] not in enum[1]):
                yield enum[0]

    def _add(self, definitions: Iterable[Definition]) -> None:
        """Mark definitions as used, and queue the code of the functions among them."""
        for definition in definitions:
            if definition not in self._used:
                self._used.add(definition)
                if definition.kind in ("function", "method"):
                    self._pending.append((definition.path, definition.line))


def _entry_objects(entry_points: Iterable[str], *, files_by_name: dict[str, list[Path]]) -> dict[Path, set[str]]:
    """
    Find the objects that the entry points of a project name.

    Args:
        entry_points: The object references of the entry points.
        files_by_name: The files of the project by their module names.

    Returns:
        The qualified names of the objects in each file. `module:Class.method` names the class, and the
        method in it if there is one.
    """
    entry_objects: dict[Path, set[str]] = defaultdict(set)
    for reference in entry_points:
        module, attributes = _entry_point_object(reference)
        for path in files_by_name.get(module, []):
            entry_objects[path].update(".".join(attributes[:index]) for index in range(1, len(attributes) + 1))
    return entry_objects


def _entry_point_object(reference: str) -> tuple[str, list[str]]:
    """
    Split the object reference of an entry point, such as `package.module:Class.method [extra]`.

    Returns:
        The module, and the names of the object in it: none for a reference that names only a module or
        that is not valid.
    """
    match = _OBJECT_REFERENCE.fullmatch(reference.strip())
    if match is None:
        return "", []
    attributes = match["attributes"]
    return match["module"], [] if attributes is None else attributes.split(".")


def _is_dunder(name: str) -> bool:
    """
    Check whether a name is a dunder, such as `__init__` or `__all__`.

    Python and tools refer to dunders implicitly, so they always count as used.
    """
    return len(name) > 4 and name.startswith("__") and name.endswith("__")
