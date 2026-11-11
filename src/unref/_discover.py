import os
from collections.abc import Iterable, Iterator
from pathlib import Path


def discover(
    paths: Iterable[str | os.PathLike[str]],
    *,
    exclude: Iterable[str | os.PathLike[str]],
) -> list[Path]:
    """
    Find the Python files of a project.

    Args:
        paths: The files and directories that make up the project. A file is taken as is, a directory
            is searched for Python files.
        exclude: The files and directories that are not part of the project. A file that is excluded,
            or is under an excluded directory, is left out.

    Returns:
        The Python files, sorted.
    """
    excluded = [Path(path).resolve() for path in exclude]

    files: set[Path] = set()
    for path in map(Path, paths):
        if path.is_dir():
            files.update(_walk(path, excluded))
        elif not _is_excluded(path, excluded):
            files.add(path)
    return sorted(files)


def _walk(directory: Path, excluded: list[Path]) -> Iterator[Path]:
    """
    Yield the Python files under a directory, skipping hidden directories such as `.venv` and the
    excluded files and directories.
    """
    for root, directories, files in os.walk(directory):
        # Pruned in place, so that the skipped directories are not walked into.
        directories[:] = [
            name for name in directories if not name.startswith(".") and not _is_excluded(Path(root, name), excluded)
        ]
        for name in files:
            if name.endswith(".py") and not _is_excluded(Path(root, name), excluded):
                yield Path(root, name)


def _is_excluded(path: Path, excluded: list[Path]) -> bool:
    """
    Check whether a path is excluded, or is under an excluded directory.
    """
    absolute = Path(path).resolve()
    return any(absolute.is_relative_to(directory) for directory in excluded)
