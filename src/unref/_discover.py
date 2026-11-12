import os
from collections.abc import Iterable, Iterator
from pathlib import Path

from pathspec import GitIgnoreSpec

# Directories that are never part of a project. The same as the ones that ruff and ty leave out by default.
DEFAULT_EXCLUDED_DIRECTORIES = frozenset(
    {
        ".bzr",
        ".direnv",
        ".eggs",
        ".git",
        ".git-rewrite",
        ".hg",
        ".ipynb_checkpoints",
        ".mypy_cache",
        ".nox",
        ".pants.d",
        ".pyenv",
        ".pytest_cache",
        ".pytype",
        ".ruff_cache",
        ".svn",
        ".tox",
        ".venv",
        ".vscode",
        "__pypackages__",
        "_build",
        "buck-out",
        "dist",
        "node_modules",
        "site-packages",
        "venv",
    }
)

# The patterns of a `.gitignore` file, with the absolute directory that they are relative to.
_IgnoreFile = tuple[Path, GitIgnoreSpec]


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
        elif not is_under(path, excluded):
            files.add(path)
    return sorted(files)


def _walk(directory: Path, excluded: list[Path]) -> Iterator[Path]:
    """
    Yield the Python files under a directory. Skips the directories that are excluded by default, such
    as `.venv`, the files and directories that git ignores, and the excluded files and directories.
    """
    # The `.gitignore` files that apply to each directory still to be walked, except its own.
    inherited = {directory: _ignore_files_above(directory)}
    for root, directories, files in os.walk(directory):
        current = Path(root)
        ignore_files = inherited.pop(current) + _read_ignore_file(current)
        # Pruned in place, so that the skipped directories are not walked into.
        directories[:] = [
            name
            for name in directories
            if name not in DEFAULT_EXCLUDED_DIRECTORIES
            and not _is_ignored(current / name, ignore_files, is_directory=True)
            and not is_under(current / name, excluded)
        ]
        inherited.update((current / name, ignore_files) for name in directories)
        for name in files:
            if (
                name.endswith(".py")
                and not _is_ignored(current / name, ignore_files, is_directory=False)
                and not is_under(current / name, excluded)
            ):
                yield current / name


def _ignore_files_above(directory: Path) -> list[_IgnoreFile]:
    """
    Read the `.gitignore` files that apply to a directory from above it: those of its parents, up to the
    root of its git repository. Outside a git repository no `.gitignore` file applies, as with git.
    """
    absolute = directory.resolve()
    candidates = [absolute, *absolute.parents]
    root = next((candidate for candidate in candidates if (candidate / ".git").exists()), None)
    if root is None:
        return []
    # The parents from the root of the repository down, without the directory itself.
    parents = reversed(absolute.parents[: candidates.index(root)])
    return [ignore_file for parent in parents for ignore_file in _read_ignore_file(parent)]


def _read_ignore_file(directory: Path) -> list[_IgnoreFile]:
    """
    Read the `.gitignore` file of a directory, if it has one.
    """
    path = directory / ".gitignore"
    if not path.is_file():
        return []
    with path.open(encoding="utf-8", errors="replace") as file:
        return [(directory.resolve(), GitIgnoreSpec.from_lines(file))]


def _is_ignored(path: Path, ignore_files: list[_IgnoreFile], *, is_directory: bool) -> bool:
    """
    Check whether git ignores a file or directory. A deeper `.gitignore` file overrides the ones above
    it, and within a file the last matching pattern wins, so that `!pattern` can bring a path back.
    """
    absolute = path.resolve()
    ignored = False
    for base, spec in ignore_files:
        relative = Path(os.path.relpath(absolute, base)).as_posix() + ("/" if is_directory else "")
        result = spec.check_file(relative)
        if result.include is not None:
            ignored = result.include
    return ignored


def is_under(path: Path, roots: list[Path]) -> bool:
    """
    Check whether a path is one of the given files, or is under one of the given directories.

    Args:
        path: The path to check.
        roots: The files and directories, resolved.
    """
    absolute = Path(path).resolve()
    return any(absolute.is_relative_to(root) for root in roots)
