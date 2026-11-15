#!/usr/bin/env python
"""
Time unref and vulture on the CPython codebase.

Run with `uv run scripts/benchmark.py`. It downloads the source of CPython, installs vulture and the last
release of unref written in Python into a temporary environment, runs each tool on the whole codebase a
few times and prints the mean time of each.
"""

import io
import sys
import tarfile
import tempfile
import time
from pathlib import Path

import httpx
import shpyx

_CPYTHON_TAG = "v3.14.0"
_VULTURE_VERSION = "2.16"
# The last release of unref written in Python.
_PYTHON_UNREF_VERSION = "0.1.1"
_RUNS = 5


def _download_cpython(destination: Path) -> Path:
    url = f"https://codeload.github.com/python/cpython/tar.gz/refs/tags/{_CPYTHON_TAG}"
    print(f"Downloading CPython {_CPYTHON_TAG}...")
    response = httpx.get(url, follow_redirects=True, timeout=120)
    response.raise_for_status()
    with tarfile.open(fileobj=io.BytesIO(response.content)) as archive:
        archive.extractall(destination, filter="data")
    return next(destination.iterdir())


def _install_tools(environment: Path) -> Path:
    print(f"Installing vulture {_VULTURE_VERSION} and unref {_PYTHON_UNREF_VERSION}...")
    shpyx.run(["uv", "venv", "--quiet", environment])
    shpyx.run(
        [
            "uv",
            "pip",
            "install",
            "--quiet",
            "--python",
            environment,
            f"vulture=={_VULTURE_VERSION}",
            f"unref=={_PYTHON_UNREF_VERSION}",
        ]
    )
    return environment / "bin"


def _time(command: list[str], cwd: Path) -> float:
    # One run to warm up the file system cache, then the mean of the others.
    shpyx.run(command, exec_dir=cwd, verify_return_code=False)
    times = []
    for _ in range(_RUNS):
        start = time.perf_counter()
        shpyx.run(command, exec_dir=cwd, verify_return_code=False)
        times.append(time.perf_counter() - start)
    return sum(times) / len(times)


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        cpython = _download_cpython(Path(directory) / "source")
        tools = _install_tools(Path(directory) / "venv")
        unref = str(Path(sys.executable).parent / "unref")
        python_unref = str(tools / "unref")

        commands = {
            "unref": [unref],
            "unref (one thread)": [unref, "--jobs", "1"],
            f"unref {_PYTHON_UNREF_VERSION} (Python)": [python_unref],
            f"unref {_PYTHON_UNREF_VERSION} (Python, one process)": [python_unref, "--jobs", "1"],
            "vulture": [str(tools / "vulture"), "."],
        }
        results = {}
        for name, command in commands.items():
            print(f"Timing {name}...")
            results[name] = _time(command, cpython)

    print(f"\n| Tool | Time to check CPython {_CPYTHON_TAG} |")
    print("| --- | --- |")
    for name, seconds in results.items():
        print(f"| {name} | {seconds:.2f}s |")


if __name__ == "__main__":
    main()
