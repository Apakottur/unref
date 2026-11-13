from pathlib import Path

from unref._external import ExternalModules
from unref._references import Binding
from unref._symbols import ModuleSymbols


class ClassHierarchy:
    """
    The classes of a project, with the base classes that they inherit from and their decorators.

    A base class is followed through the imports of the project, so that a class is known to inherit
    from the base classes of its base classes, wherever they are defined. A base class that is imported
    from outside the project is followed through the source of its module, such as `enum.StrEnum` to
    `enum.Enum`. A decorator is followed through the imports as well, to where it is defined.
    """

    def __init__(
        self, modules: dict[Path, ModuleSymbols], *, files_by_name: dict[str, list[Path]], external: ExternalModules
    ) -> None:
        """
        Args:
            modules: The symbols of each module of the project.
            files_by_name: The files of the project by their module name.
            external: The modules that are imported from outside the project.
        """
        self._project = modules
        # The modules outside the project that are read are added to this, so it is a copy.
        self._modules = dict(modules)
        self._files_by_name = files_by_name
        self._external = external
        self._ancestors: dict[tuple[Path, str], frozenset[str]] = {}
        self._inherited: dict[tuple[Path, str], frozenset[str]] = {}
        self._resolved_decorators: dict[tuple[Path, str], frozenset[str]] = {}
        self._resolved: dict[tuple[Path, str], frozenset[str]] = {}

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
            for base in self._modules[path].bases[name]:
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
                for file, symbol in self._locate(ancestor):
                    # The members of the modules of the project are not collected.
                    if file not in self._project:
                        found |= self._modules[file].members.get(symbol, set())
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
            self._resolved_decorators[key] = frozenset().union(
                *(self.resolve(path, decorator) for decorator in self._modules[path].decorators[name])
            )
        return self._resolved_decorators[key]

    def resolve(self, path: Path, written: str) -> frozenset[str]:
        """
        Get the dotted names of a name as it is written in a module, such as the decorator of a function.

        Args:
            path: The file of the module.
            written: The dotted name as it is written, such as `fixture` after `from pytest import fixture`.

        Returns:
            The dotted names, such as `pytest.fixture`. A name that is imported through several modules
            has the name of each.
        """
        key = (path, written)
        if key not in self._resolved:
            self._resolved[key] = frozenset(self._resolve(path, written, set()))
        return self._resolved[key]

    def _resolve(self, path: Path, written: str, seen: set[str]) -> set[str]:
        """
        Get the dotted names of a base class or decorator, as it is written in a module, and of the
        base classes of a base class.
        """
        module = self._modules[path]
        first, _, rest = written.partition(".")

        if first in module.bases:
            # A class or function of the module itself.
            return self._follow(f"{module.name.name}.{written}", seen)

        if first in module.bindings:
            # A name that the module imports.
            imported = self._imported(path, module.bindings[first])
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
        for path, symbol in self._locate(name):
            module = self._modules[path]
            first, _, rest = symbol.partition(".")
            if symbol in module.bases:
                for base in module.bases[symbol]:
                    found |= self._resolve(path, base, seen)
            elif first in module.bindings:
                imported = self._imported(path, module.bindings[first])
                if imported is not None:
                    found |= self._follow(".".join(filter(None, [imported, rest])), seen)
            elif first not in module.bases:
                # A name that the module may star import.
                for source, level in module.stars:
                    star_module = module.name.resolve(source, level)
                    if star_module is not None:
                        found |= self._follow(f"{star_module}.{symbol}", seen)
                # A name that the `__getattr__` of the module imports, which it gives out.
                for binding in module.lazy.get(first, []):
                    imported = self._imported(path, binding)
                    if imported is not None:
                        found |= self._follow(".".join(filter(None, [imported, rest])), seen)
        return found

    def _locate(self, name: str) -> list[tuple[Path, str]]:
        """
        Get the files of the module that a dotted name is in, each with the rest of the name in the
        module, such as `Outer.Inner` for a nested class.

        The longest start of the name that is a module of the project is taken, and otherwise the
        longest one that is a module outside the project. A module outside the project without a source,
        such as a compiled module, has no file, and a shorter start is not taken in its place.
        """
        parts = name.split(".")
        starts = [(".".join(parts[:index]), ".".join(parts[index:])) for index in range(len(parts) - 1, 0, -1)]
        for module, rest in starts:
            if module in self._files_by_name:
                return [(file, rest) for file in self._files_by_name[module]]
        for module, rest in starts:
            if self._external.exists(module):
                return [(file, rest) for file in self._read_external(module)]
        return []

    def _imported(self, path: Path, binding: Binding) -> str | None:
        """
        Get the dotted name that an imported name of a module stands for.
        """
        module, level, name = binding
        source = self._modules[path].name.resolve(module, level)
        if source is None:
            return None
        return ".".join(filter(None, [source, name]))

    def _read_external(self, module: str) -> list[Path]:
        """
        Get the file of a module that is imported from outside the project, and add its symbols to
        those of the project.
        """
        external = self._external.get(module)
        if external is None:
            return []
        self._modules.setdefault(external.path, external)
        return [external.path]
