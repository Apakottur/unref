#!/usr/bin/env python
"""
Set the version of unref in the files that name it.

Run with `uv run --no-project scripts/set_version.py [VERSION] [--package]`, with a version such as `1.2.3`, or
without one for the latest release on PyPI. The pre-commit examples of the docs and the benchmark name a release. With
`--package`, the version of the package in `Cargo.toml` is set as well, as the release workflow does from its tag.
"""

import argparse
import json
import re
import urllib.request
from pathlib import Path

_ROOT = Path(__file__).parent.parent
# Each file, with the pattern of the version in it. The part before the version is kept.
_RELEASES = {
    "README.md": r"(repo: https://github\.com/Apakottur/unref\s+rev: v)[0-9.]+",
    "docs/usage.md": r"(repo: https://github\.com/Apakottur/unref\s+rev: v)[0-9.]+",
    "scripts/benchmark.py": r"(\"unref==)[0-9.]+",
}
_PACKAGE = {"Cargo.toml": r'(?m)(^version = ")[^"]*'}


def _latest_version() -> str:
    with urllib.request.urlopen("https://pypi.org/pypi/unref/json", timeout=30) as response:
        return str(json.load(response)["info"]["version"])


def _set(path: str, pattern: str, version: str) -> None:
    """Set the version that the first match of a pattern names in a file, keeping its line endings."""
    file = _ROOT / path
    with file.open(newline="") as stream:
        text = stream.read()
    text, count = re.subn(pattern, rf"\g<1>{version}", text, count=1)
    if count != 1:
        raise SystemExit(f"No version of unref found in {path}.")
    with file.open("w", newline="") as stream:
        stream.write(text)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("version", nargs="?", help="the version, by default the latest release on PyPI")
    parser.add_argument("--package", action="store_true", help="also set the version of the package")
    arguments = parser.parse_args()

    version = arguments.version or _latest_version()
    files = _RELEASES | _PACKAGE if arguments.package else _RELEASES
    for path, pattern in files.items():
        _set(path, pattern, version)
    print(f"Set the version to {version}.")


if __name__ == "__main__":
    main()
