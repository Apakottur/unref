#!/usr/bin/env python
"""
Set the version of unref in the files that name it.

Run with `uv run --only-group dev scripts/set_version.py [VERSION]`, with a version such as `1.2.3`, or without one
for the latest release on PyPI. The pre-commit examples of the docs and the benchmark name a release, and the package
has its version, so that a build from source names the release that it follows.
"""

import re
from pathlib import Path
from typing import Annotated

import httpx
import typer

_ROOT = Path(__file__).parent.parent
# Each file, with the pattern of the version in it. The part before the version is kept. A checkout on
# Windows has CRLF line endings.
_RELEASES = {
    "README.md": r"(repo: https://github\.com/Apakottur/unref-pre-commit\s+rev: v)[0-9.]+",
    "docs/usage.md": r"(repo: https://github\.com/Apakottur/unref-pre-commit\s+rev: v)[0-9.]+",
    "scripts/benchmark.py": r"(\"unref==)[0-9.]+",
    "Cargo.toml": r'(?m)(^version = ")[^"]*',
    "Cargo.lock": r'(name = "unref"\r?\nversion = ")[^"]*',
}

app = typer.Typer(add_completion=False)


def _latest_version() -> str:
    response = httpx.get("https://pypi.org/pypi/unref/json", timeout=30)
    response.raise_for_status()
    return str(response.json()["info"]["version"])


def _set(path: str, pattern: str, version: str) -> None:
    """Set the version that the first match of a pattern names in a file, keeping its line endings."""
    file = _ROOT / path
    with file.open(newline="") as stream:
        text = stream.read()
    text, count = re.subn(pattern, rf"\g<1>{version}", text, count=1)
    if count != 1:
        print(f"No version of unref found in {path}.")
        raise typer.Exit(1)
    with file.open("w", newline="") as stream:
        stream.write(text)


@app.command()
def main(
    version: Annotated[str | None, typer.Argument(help="The version, by default the latest release on PyPI.")] = None,
) -> None:
    """Set the version of unref in the docs, the benchmark and the package."""
    version = version or _latest_version()
    for path, pattern in _RELEASES.items():
        _set(path, pattern, version)
    print(f"Set the version to {version}.")


if __name__ == "__main__":
    app()
