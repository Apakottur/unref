import ast
import warnings
from dataclasses import dataclass
from importlib.machinery import BYTECODE_SUFFIXES, EXTENSION_SUFFIXES, SOURCE_SUFFIXES
from pathlib import Path

from unref._definitions import Definition, find_definitions
from unref._modules import ModuleName
from unref._references import Binding, References, find_references

# The files that a module can be imported from, in the order that Python looks for them: a compiled
# module, a source, and a bytecode file.
_SUFFIXES = (*EXTENSION_SUFFIXES, *SOURCE_SUFFIXES, *BYTECODE_SUFFIXES)


@dataclass(frozen=True)
class ExternalModule:
    """
    A module that is imported from outside the project, such as a module of the standard library.

    Attributes:
        path: The source file of the module.
        name: The name that the module is imported by.
        definitions: What the module defines.
        bindings: The names that the module imports at its top level.
        stars: The modules that the module star imports at its top level, each as it is written and
            with its number of leading dots.
        lazy: The names that the module gives out from its `__getattr__`, each with the imports of it.
    """

    path: Path
    name: ModuleName
    definitions: list[Definition]
    bindings: dict[str, Binding]
    stars: list[tuple[str | None, int]]
    lazy: dict[str, list[Binding]]


class ExternalModules:
    """
    The modules that are imported from outside the project.

    A module is found on a search path the way Python finds it, and read from its source, but it is
    never imported: its code does not run. A module without a Python source, such as a builtin or a
    compiled module, is not read.
    """

    def __init__(self, search_path: list[Path]) -> None:
        """
        Args:
            search_path: The directories that modules are imported from, in order.
        """
        self._search_path = search_path
        self._modules: dict[str, ExternalModule | None] = {}

    def get(self, name: str) -> ExternalModule | None:
        """
        Get a module by its dotted name.

        Args:
            name: The dotted name of the module, such as `logging.handlers`.

        Returns:
            The module, or None if it is not found or does not parse.
        """
        if name not in self._modules:
            self._modules[name] = self._read(name)
        return self._modules[name]

    def _read(self, name: str) -> ExternalModule | None:
        """
        Find a module and read its source.
        """
        # Each package of the name is looked for in turn, in the directories of the package above it.
        locations: list[Path] | None = self._search_path
        file: Path | None = None
        for part in name.split("."):
            if locations is None:
                # A module that is not a package has no submodules.
                return None
            found = _find(part, locations)
            if found is None:
                return None
            file, locations = found

        if file is None or file.suffix != ".py":
            return None
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                tree = ast.parse(file.read_bytes(), filename=str(file))
        except (OSError, SyntaxError, ValueError):
            return None

        references = find_references(tree)
        return ExternalModule(
            path=file,
            name=ModuleName(name=name, is_package=locations is not None),
            definitions=find_definitions(file, tree),
            bindings=references.bindings,
            stars=star_imports(references),
            lazy=lazy_imports(tree, references),
        )


def star_imports(references: References) -> list[tuple[str | None, int]]:
    """
    Get the modules that a module star imports at its top level.

    Args:
        references: What the module refers to.

    Returns:
        The modules, each as it is written and with its number of leading dots.
    """
    return sorted(
        ((source, level) for owner, source, level, name in references.imports if owner is None and name == "*"),
        key=str,
    )


def lazy_imports(tree: ast.Module, references: References) -> dict[str, list[Binding]]:
    """
    Get the names that a module gives out from its `__getattr__` (PEP 562), which imports them when they
    are first read, such as `ThreadPoolExecutor` in `concurrent.futures`.

    A name is given out when the `__getattr__` of the module imports it with `from module import name`.
    The code of the function is not run, so the condition that the import is under is not checked.

    Args:
        tree: The parsed module.
        references: What the module refers to.

    Returns:
        The names, each with the imports of it in `__getattr__`, as the module imports them.
    """
    # The last definition of `__getattr__` at the top level of the module is the one that counts.
    lines = [
        node.lineno
        for node in tree.body
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == "__getattr__"
    ]
    if not lines:
        return {}
    lazy: dict[str, list[Binding]] = {}
    for owner, source, level, name in sorted(references.imports, key=str):
        if owner == lines[-1] and name != "*":
            lazy.setdefault(name, []).append((source, level, name))
    return lazy


def environment_paths(environment: Path) -> list[Path]:
    """
    Get the directories that a Python environment imports its installed modules from, without running
    its interpreter.

    They are its `site-packages`, followed by the directories that the `.pth` files there add, in the
    order that Python's `site` module adds them. A line of a `.pth` file that imports code is not run.

    Args:
        environment: The environment, as ty takes it: a virtual environment such as `.venv`, a
            `sys.prefix` such as `/usr`, or an interpreter such as `.venv/bin/python`.

    Returns:
        The directories that exist, in order.
    """
    if environment.is_file():
        # An interpreter is in the `bin` or `Scripts` directory of its environment, or, for a system
        # Python on Windows, directly in it. Its link is not followed: the interpreter of a virtual
        # environment links to the one it was created from.
        environment = environment.parent
        if environment.name in ("bin", "Scripts"):
            environment = environment.parent

    paths: list[Path] = []
    # `lib/python3.X/site-packages` on POSIX, and `Lib/site-packages` on Windows.
    for site_packages in sorted(
        {*environment.glob("lib*/python3*/site-packages/"), *environment.glob("Lib/site-packages/")}
    ):
        paths.append(site_packages)
        for pth in sorted(site_packages.glob("*.pth")):
            try:
                lines = pth.read_text(encoding="utf-8-sig", errors="replace").splitlines()
            except OSError:
                continue
            for line in lines:
                if not line.strip() or line.startswith(("#", "import ", "import\t")):
                    continue
                directory = site_packages / line.rstrip()
                if directory.is_dir():
                    paths.append(directory)
    # `lib64` is often a link to `lib`.
    return list(dict.fromkeys(path.resolve() for path in paths))


def _find(name: str, locations: list[Path]) -> tuple[Path | None, list[Path] | None] | None:
    """
    Find a module in the directories that it can be imported from, in the order that Python looks for
    it.

    Args:
        name: The name of the module in its package, without dots.
        locations: The directories of the search path, or of the package of the module.

    Returns:
        The file of the module, and the directories of its submodules if it is a package. A namespace
        package, which is a directory without an `__init__.py`, has no file, and is made of the
        directories of its name in every location. None if the module is not found.
    """
    portions: list[Path] = []
    for location in locations:
        directory = location / name
        if directory.is_dir():
            for suffix in _SUFFIXES:
                if (directory / f"__init__{suffix}").is_file():
                    return directory / f"__init__{suffix}", [directory]
            portions.append(directory)
        for suffix in _SUFFIXES:
            if (location / f"{name}{suffix}").is_file():
                return location / f"{name}{suffix}", None
    if portions:
        return None, portions
    return None
