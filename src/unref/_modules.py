from collections.abc import Collection
from dataclasses import dataclass
from pathlib import Path

_INIT = "__init__.py"


@dataclass(frozen=True)
class ModuleName:
    """
    The name that a Python file is imported by.

    Attributes:
        name: The dotted name, such as `package.module`. The `__init__.py` of a package has the name
            of the package.
        is_package: Whether the file is the `__init__.py` of a package.
    """

    name: str
    is_package: bool

    def resolve(self, module: str | None, level: int) -> str | None:
        """
        Get the dotted name of the module that an import in this module imports from.

        Args:
            module: The module of the import, as it is written: `helpers` in `from .helpers import name`,
                and None in `from . import name`.
            level: The number of leading dots of the import, 0 for an absolute import.

        Returns:
            The dotted name, or None if the import has more dots than there are packages.
        """
        if level == 0:
            return module

        # One dot is the package of this module, and each further dot is one package up.
        package = self.name.split(".") if self.is_package else self.name.split(".")[:-1]
        if level > len(package):
            return None
        return ".".join([*package[: len(package) - level + 1], *filter(None, [module])])


def module_name(path: Path, roots: Collection[Path] = ()) -> ModuleName:
    """
    Get the name that a Python file is imported by.

    The name is made of the file and of the directories above it that are packages. A directory is a
    package if it has an `__init__.py`, or if it is a namespace package: a directory without one below
    a package, both under one of the given directories. The first directory that is not a package is
    taken to be on the import path.

    Args:
        path: The file.
        roots: The files and directories that make up the project, resolved. A namespace package is only
            looked for under them, so that a package outside the project does not pull directories in.
    """
    path = path.resolve()
    is_package = path.name == _INIT
    parts = [] if is_package else [path.stem]

    directory = path.parent
    while (directory / _INIT).is_file() or _is_namespace_package(directory, roots):
        parts.insert(0, directory.name)
        directory = directory.parent

    return ModuleName(name=".".join(parts), is_package=is_package)


def _is_namespace_package(directory: Path, roots: Collection[Path]) -> bool:
    """
    Check whether a directory without an `__init__.py` is a namespace package of the project, which is
    to say that it is below a package with one, both under one of the given directories.
    """
    return any(
        (ancestor / _INIT).is_file()
        for root in roots
        if directory.is_relative_to(root)
        for ancestor in directory.parents
        if ancestor.is_relative_to(root)
    )


def import_root(path: Path, name: ModuleName) -> Path:
    """
    Get the directory on the import path that a Python file is imported from.

    Args:
        path: The file.
        name: The name that the file is imported by.

    Returns:
        The directory that holds the first package of the name, or the file itself if it is not in a
        package.
    """
    depth = name.name.count(".") + (1 if name.is_package else 0)
    return path.resolve().parents[depth]
