import os
from collections.abc import Iterable, Iterator
from pathlib import Path

from pathspec import GitIgnoreSpec

# The patterns that are excluded by default, before the `exclude` ones. The same as ty's defaults.
DEFAULT_EXCLUDE = (
    "**/.bzr/",
    "**/.direnv/",
    "**/.eggs/",
    "**/.git/",
    "**/.git-rewrite/",
    "**/.hg/",
    "**/.mypy_cache/",
    "**/.nox/",
    "**/.pants.d/",
    "**/.pytype/",
    "**/.ruff_cache/",
    "**/.svn/",
    "**/.tox/",
    "**/.venv/",
    "**/__pypackages__/",
    "**/_build/",
    "**/buck-out/",
    "**/dist/",
    "**/node_modules/",
    "**/venv/",
)

# The patterns of a `.gitignore` file, with the absolute directory that they are relative to.
_IgnoreFile = tuple[Path, GitIgnoreSpec]


def discover(
    paths: Iterable[str | os.PathLike[str]],
    *,
    exclude: Iterable[str],
) -> list[Path]:
    """
    Find the Python files of a project.

    Args:
        paths: The files and directories that make up the project. A file is taken as is, a directory
            is searched for Python files.
        exclude: The patterns of the files and directories that are not part of the project, on top of
            the default ones, in the syntax of `.gitignore`. They are anchored at the current directory.

    Returns:
        The Python files, sorted.
    """
    excluded = _Exclude(exclude)

    files: set[Path] = set()
    for path in map(Path, paths):
        if path.is_dir():
            files.update(_walk(path, excluded))
        else:
            # A file that is given directly is always checked, as with ruff and ty.
            files.add(path)
    return sorted(files)


class _Exclude:
    """
    The `exclude` patterns, with the default ones before them.
    """

    def __init__(self, patterns: Iterable[str]) -> None:
        self._spec = GitIgnoreSpec.from_lines([*DEFAULT_EXCLUDE, *map(_anchor, patterns)])
        # The patterns are anchored at the current directory.
        self._root = Path.cwd().resolve()

    def __call__(self, path: Path, *, is_directory: bool) -> bool:
        """
        Check whether a file or directory is excluded. The last pattern that matches it decides.
        """
        absolute = path.resolve()
        # A path outside of the current directory can still match the patterns that start with `**/`.
        base = self._root if absolute.is_relative_to(self._root) else Path(absolute.anchor)
        relative = absolute.relative_to(base).as_posix() + ("/" if is_directory else "")
        return bool(self._spec.check_file(relative).include)


def _anchor(pattern: str) -> str:
    """
    Anchor a pattern at the current directory, as ty anchors them at the project root: `src` matches
    only the `src` there, `**/src` matches one at any depth.
    """
    negation, body = ("!", pattern[1:]) if pattern.startswith("!") else ("", pattern)
    body = body.removeprefix("./")
    if not body.startswith(("/", "**/")):
        body = "/" + body
    return negation + body


def _walk(directory: Path, excluded: _Exclude) -> Iterator[Path]:
    """
    Yield the Python files under a directory. Skips the excluded files and directories, and the ones that
    git ignores.
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
            if not excluded(current / name, is_directory=True)
            and not _is_ignored(current / name, ignore_files, is_directory=True)
        ]
        inherited.update((current / name, ignore_files) for name in directories)
        for name in files:
            if (
                name.endswith(".py")
                and not excluded(current / name, is_directory=False)
                and not _is_ignored(current / name, ignore_files, is_directory=False)
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
