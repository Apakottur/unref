import ast
import math
import os
import warnings
from collections.abc import Sequence
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from pathlib import Path

from unref._definitions import Definition
from unref._external import lazy_imports
from unref._ignores import IgnoreComment, find_ignore_comments
from unref._models import Finding
from unref._references import Binding, References, walk_module

# Below this much source code, the work of a check is shorter than starting the processes.
MIN_PARALLEL_BYTES = 1_000_000

# The number of batches per process, so that a process that gets large files does not hold up the rest.
_BATCHES_PER_PROCESS = 4


@dataclass(frozen=True)
class FileFacts:
    """
    What a Python file defines and refers to.

    Attributes:
        definitions: The definitions of the file.
        references: What the file refers to.
        ignore_comments: The ignore comments of the file that parse.
        ignore_findings: The findings about the ignore comments of the file, which they can ignore
            themselves.
        lazy: The names that the file gives out from its `__getattr__`, each with the imports of it.
    """

    definitions: list[Definition]
    references: References
    ignore_comments: list[IgnoreComment]
    ignore_findings: list[Finding]
    lazy: dict[str, list[Binding]]


def analyze_files(paths: Sequence[Path], *, jobs: int | None) -> dict[Path, FileFacts | Finding]:
    """
    Find what each Python file defines and refers to.

    Each file is analyzed on its own, so that a large project is analyzed by several processes at once.

    Args:
        paths: The files.
        jobs: The number of processes, or None for one for each CPU. A small project is analyzed in the
            current process.

    Returns:
        For each file, what it defines and refers to, or a finding if it does not parse.

    Raises:
        ValueError: If the number of processes is less than 1.
    """
    if jobs is not None and jobs < 1:
        msg = f"The number of jobs must be at least 1: {jobs}"
        raise ValueError(msg)

    if jobs == 1 or sum(path.stat().st_size for path in paths) < MIN_PARALLEL_BYTES:
        return {path: analyze_file(path) for path in paths}

    processes = jobs or os.cpu_count() or 1
    chunk_size = math.ceil(len(paths) / (processes * _BATCHES_PER_PROCESS))
    with ProcessPoolExecutor(max_workers=processes) as executor:
        return dict(zip(paths, executor.map(analyze_file, paths, chunksize=chunk_size), strict=True))


def analyze_file(path: Path) -> FileFacts | Finding:
    """
    Find what a Python file defines and refers to.

    Args:
        path: The file.

    Returns:
        What the file defines and refers to, or a finding if it does not parse.
    """
    source = path.read_bytes()
    try:
        tree = _parse(source, path)
    except SyntaxError as error:
        return Finding(
            path=path,
            line=error.lineno or 1,
            column=error.offset or 1,
            code="invalid-syntax",
            name=error.msg,
        )

    definitions, references = walk_module(path, tree, source)
    ignore_comments, ignore_findings = find_ignore_comments(source, path)
    return FileFacts(
        definitions=definitions,
        references=references,
        ignore_comments=ignore_comments,
        ignore_findings=ignore_findings,
        lazy=lazy_imports(tree, references),
    )


def _parse(source: bytes, path: Path) -> ast.Module:
    """
    Parse a Python module.

    The warnings that parsing emits, such as `SyntaxWarning` for an invalid escape sequence, are about
    the code of the project, not about the check, so they are not shown.

    Args:
        source: The content of the module.
        path: The file of the module.

    Returns:
        The syntax tree of the module.

    Raises:
        SyntaxError: The module does not parse.
    """
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return ast.parse(source, filename=str(path))
