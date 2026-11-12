import os
from collections.abc import Iterable

from unref._engine import find_unused
from unref._models import Finding, Override


def check(
    paths: Iterable[str | os.PathLike[str]],
    *,
    used_names: Iterable[str] = (),
    used_decorators: Iterable[str] = (),
    exclude: Iterable[str | os.PathLike[str]] = (),
    overrides: Iterable[Override] = (),
    jobs: int | None = 1,
    python: str | os.PathLike[str] | None = None,
) -> list[Finding]:
    """
    Find the unused code in a Python project.

    Args:
        paths: The files and directories that make up the project.
        used_names: The names of the definitions that count as used, as glob patterns such as `test_*`.
        used_decorators: The decorators that make the definitions they decorate count as used, as
            dotted names. For example, `app.command` covers `@app.command` and `@app.command("name")`.
        exclude: The files and directories that are not part of the project. They are not checked, and
            a reference from them does not count.
        overrides: Options that apply only to some of the files of the project, on top of the options
            above. For example, `Override(paths=["tests"], used_names=["test_*"])`.
        jobs: The number of processes that analyze the files, or None for one for each CPU. A small
            project is always analyzed in the current process. With more than one process, a script
            that calls this function must guard its code with `if __name__ == "__main__":`, as
            [`multiprocessing`](https://docs.python.org/3/library/multiprocessing.html) requires.
        python: The Python environment that the project runs in, as a virtual environment such as
            `.venv`, a `sys.prefix` such as `/usr`, or an interpreter such as `.venv/bin/python`, as
            ty takes it. A base class that is installed there is followed through its source, which is
            read without being imported. The site-packages of the environment replace those of the
            environment that runs the check. Defaults to the environment that runs the check.

    Returns:
        The findings, one for each unused definition, sorted by file and line.
    """
    return find_unused(
        paths,
        used_names=used_names,
        used_decorators=used_decorators,
        exclude=exclude,
        overrides=overrides,
        jobs=jobs,
        python=python,
    )
