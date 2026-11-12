from pathlib import Path

from unref._external import ExternalModules
from unref._modules import ModuleName
from unref._references import Binding


class ClassHierarchy:
    """
    The classes of a project, with the base classes that they inherit from and their decorators.

    A base class is followed through the imports of the project, so that a class is known to inherit
    from the base classes of its base classes, wherever they are defined. A base class that is imported
    from outside the project is followed through the source of its module, such as `enum.StrEnum` to
    `enum.Enum`. A decorator is followed through the imports as well, to where it is defined.
    """

    def __init__(
        self,
        *,
        names: dict[Path, ModuleName],
        files_by_name: dict[str, list[Path]],
        bindings: dict[Path, dict[str, Binding]],
        stars: dict[Path, list[tuple[str | None, int]]],
        classes: dict[Path, dict[str, tuple[str, ...]]],
        decorators: dict[Path, dict[str, tuple[str, ...]]],
        external: ExternalModules,
    ) -> None:
        """
        Args:
            names: The name of each module of the project.
            files_by_name: The files of the project by their module name.
            bindings: The names that each module imports at its top level.
            stars: The modules that each module star imports at its top level, each as it is written
                and with its number of leading dots.
            classes: The classes and functions of each module, by their name in the module
                (`Outer.Inner` for a nested class), with their base classes as they are written. A
                function has none.
            decorators: The classes of each module, by their name in the module, with their decorators
                as they are written.
            external: The modules that are imported from outside the project.
        """
        self._decorators = decorators
        self._resolved_decorators: dict[tuple[Path, str], frozenset[str]] = {}
        # The external modules that are read are added to these, so they are copies.
        self._names = dict(names)
        self._files_by_name = files_by_name
        self._bindings = dict(bindings)
        self._stars = dict(stars)
        self._classes = dict(classes)
        self._ancestors: dict[tuple[Path, str], frozenset[str]] = {}
        self._external = external
        # The members of the classes of each module outside the project, by the name of the class.
        self._external_members: dict[Path, dict[str, set[str]]] = {}
        self._inherited: dict[tuple[Path, str], frozenset[str]] = {}

    def ancestors(self, path: Path, name: str) -> frozenset[str]:
        """
        Get the base classes that a class inherits from, directly or not.

        Args:
            path: The file of the class.
            name: The name of the class in its module.

        Returns:
            The dotted names of the base classes, such as `pydantic.BaseModel`. A base class that is
            imported through several modules has the name of each.
        """
        key = (path, name)
        if key not in self._ancestors:
            seen: set[str] = set()
            found: set[str] = set()
            for base in self._classes[path][name]:
                found |= self._resolve(path, base, seen)
            self._ancestors[key] = frozenset(found)
        return self._ancestors[key]

    def external_members(self, path: Path, name: str) -> frozenset[str]:
        """
        Get the names of the members that a class inherits from the base classes outside the project.

        Code outside the project can use such a member, such as `logging.Handler` calling `emit`, so a
        member of the class with the same name overrides one that is used.

        Args:
            path: The file of the class.
            name: The name of the class in its module.

        Returns:
            The names of the methods and attributes that the base classes outside the project define,
            in their body or on their instances.
        """
        key = (path, name)
        if key not in self._inherited:
            found: set[str] = set()
            for ancestor in self.ancestors(path, name):
                module, _, symbol = ancestor.rpartition(".")
                if module not in self._files_by_name:
                    for file in self._read_external(module):
                        found |= self._external_members.get(file, {}).get(symbol, set())
            self._inherited[key] = frozenset(found)
        return self._inherited[key]

    def decorators(self, path: Path, name: str) -> frozenset[str]:
        """
        Get the decorators of a class.

        Args:
            path: The file of the class.
            name: The name of the class in its module.

        Returns:
            The dotted names of the decorators, such as `dataclasses.dataclass`. A decorator that is
            imported through several modules has the name of each.
        """
        key = (path, name)
        if key not in self._resolved_decorators:
            found: set[str] = set()
            for decorator in self._decorators[path][name]:
                found |= self._resolve(path, decorator, set())
            self._resolved_decorators[key] = frozenset(found)
        return self._resolved_decorators[key]

    def _resolve(self, path: Path, written: str, seen: set[str]) -> set[str]:
        """
        Get the dotted names of a base class or decorator, as it is written in a module, and of the
        base classes of a base class.
        """
        first, _, rest = written.partition(".")

        if first in self._classes[path]:
            # A class or function of the module itself.
            return self._follow(f"{self._names[path].name}.{written}", seen)

        if first in self._bindings[path]:
            # A name that the module imports.
            imported = self._imported(path, self._bindings[path][first])
            if imported is None:
                return set()
            return self._follow(".".join(filter(None, [imported, rest])), seen)

        # A builtin, or a name that the module does not define at its top level.
        return {written}

    def _follow(self, name: str, seen: set[str]) -> set[str]:
        """
        Get a dotted name, and the dotted names of what it stands for in the project: the base classes
        of a class, and the origin of an imported name.
        """
        if name in seen:
            return set()
        seen.add(name)

        found = {name}
        module, _, symbol = name.rpartition(".")
        for path in self._files_by_name.get(module) or self._read_external(module):
            if symbol in self._classes[path]:
                for base in self._classes[path][symbol]:
                    found |= self._resolve(path, base, seen)
            elif symbol in self._bindings[path]:
                imported = self._imported(path, self._bindings[path][symbol])
                if imported is not None:
                    found |= self._follow(imported, seen)
            else:
                # A name that the module may star import.
                for source, level in self._stars[path]:
                    star_module = self._names[path].resolve(source, level)
                    if star_module is not None:
                        found |= self._follow(f"{star_module}.{symbol}", seen)
        return found

    def _imported(self, path: Path, binding: Binding) -> str | None:
        """
        Get the dotted name that an imported name of a module stands for.
        """
        module, level, name = binding
        source = self._names[path].resolve(module, level)
        if source is None:
            return None
        return ".".join(filter(None, [source, name]))

    def _read_external(self, module: str) -> list[Path]:
        """
        Get the file of a module that is imported from outside the project, and add its classes and
        imports to those of the project.
        """
        external = self._external.get(module)
        if external is None:
            return []
        path = external.path
        if path not in self._classes:
            self._names[path] = external.name
            self._bindings[path] = external.bindings
            self._stars[path] = external.stars
            self._classes[path] = {
                definition.qualified_name: definition.bases
                for definition in external.definitions
                if definition.kind in ("class", "function")
            }
            members: dict[str, set[str]] = {}
            for definition in external.definitions:
                if definition.owner is not None:
                    members.setdefault(definition.owner, set()).add(definition.name)
            self._external_members[path] = members
        return [path]
