import os
from collections.abc import Iterable

from unref._engine import find_unused
from unref._models import DEFAULT_DUMMY_VARIABLE_RGX, FRAMEWORKS, Finding, Override


def check(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_names: Iterable[str] = (),
    used_decorators: Iterable[str] = (),
    exclude: Iterable[str] = (),
    overrides: Iterable[Override] = (),
    jobs: int | None = 1,
    python: str | os.PathLike[str] | None = None,
    dummy_variable_rgx: str = DEFAULT_DUMMY_VARIABLE_RGX,
    entry_points: Iterable[str] = (),
    frameworks: Iterable[str] = FRAMEWORKS,
) -> list[Finding]:
    """
    Find the unused code in a Python project.

    The options are described in https://unref.readthedocs.io/en/latest/configuration/.

    Args:
        paths: The files and directories that make up the project.
        used_names: Names of the definitions that count as used, as glob patterns.
        used_decorators: Decorators that make the definitions they decorate count as used.
        exclude: Patterns of the files and directories that are not part of the project.
        overrides: Options that apply only to some of the definitions of the project.
        jobs: Number of processes that analyze the files, or None for one for each CPU. More than one
            needs an `if __name__ == "__main__":` guard, as `multiprocessing` does.
        python: Python environment or interpreter that the project runs in. Defaults to the one that
            runs the check.
        dummy_variable_rgx: Regular expression for the names of the variables that are not used on
            purpose. Defaults to Ruff's default.
        entry_points: Object references of the entry points of the project, such as
            `package.module:function`. The objects they name count as used.
        frameworks: Test frameworks whose tests, fixtures and hooks count as used: `pytest` and
            `unittest`. Defaults to both.

    Returns:
        The findings, one for each unused definition, sorted by file and line.

    Raises:
        ValueError: If a framework is not a known one.
    """
    return find_unused(
        paths,
        used_names=used_names,
        used_decorators=used_decorators,
        exclude=exclude,
        overrides=overrides,
        jobs=jobs,
        python=python,
        dummy_variable_rgx=dummy_variable_rgx,
        entry_points=entry_points,
        frameworks=frameworks,
    )
