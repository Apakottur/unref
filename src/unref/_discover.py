import os
from collections.abc import Iterable, Iterator
from pathlib import Path


def discover(paths: Iterable[str | os.PathLike[str]]) -> list[Path]:
    """
    Find the Python files of a project.

    Args:
        paths: The files and directories that make up the project. A file is taken as is, a directory
            is searched for Python files.

    Returns:
        The Python files, sorted.
    """
    files: set[Path] = set()
    for path in map(Path, paths):
        if path.is_dir():
            files.update(_walk(path))
        else:
            files.add(path)
    return sorted(files)


def _walk(directory: Path) -> Iterator[Path]:
    """
    Yield the Python files under a directory, skipping hidden directories such as `.venv`.
    """
    for root, directories, files in os.walk(directory):
        # Pruned in place, so that the hidden directories are not walked into.
        directories[:] = [name for name in directories if not name.startswith(".")]
        for name in files:
            if name.endswith(".py"):
                yield Path(root, name)
