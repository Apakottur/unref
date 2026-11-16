#!/usr/bin/env python
"""
Time unref and other dead code finders on the CPython codebase.

Run with `uv run scripts/benchmark.py`. It downloads the source of CPython, installs each other tool into its own
temporary environment, runs each tool on the whole codebase a few times, prints the mean time of each and draws
them as a bar chart in `assets/benchmark.svg`. A tool that takes longer than the time limit is shown as over it.
"""

import io
import subprocess
import sys
import tarfile
import tempfile
import time
from pathlib import Path

import httpx
import shpyx

_CPYTHON_TAG = "v3.14.0"
_TOOLS = {
    "vulture": ("vulture==2.16", ["."]),
    "deadcode": ("deadcode==2.6.3", ["."]),
    "skylos": ("skylos==5.6.0", [".", "--no-upload", "--no-cache"]),
}
_RUNS = 5
_TIME_LIMIT = 30.0
_CHART = Path(__file__).parent.parent / "assets" / "benchmark.svg"


def _download_cpython(destination: Path) -> Path:
    url = f"https://codeload.github.com/python/cpython/tar.gz/refs/tags/{_CPYTHON_TAG}"
    print(f"Downloading CPython {_CPYTHON_TAG}...")
    response = httpx.get(url, follow_redirects=True, timeout=120)
    response.raise_for_status()
    with tarfile.open(fileobj=io.BytesIO(response.content)) as archive:
        archive.extractall(destination, filter="data")
    return next(destination.iterdir())


def _install(name: str, requirement: str, environment: Path) -> Path:
    print(f"Installing {requirement}...")
    shpyx.run(["uv", "venv", "--quiet", environment])
    shpyx.run(["uv", "pip", "install", "--quiet", "--python", environment, requirement])
    return environment / "bin" / name


def _run(command: list[str], cwd: Path) -> float | None:
    """Run a command once and return how long it took, or `None` if it went over the time limit."""
    start = time.perf_counter()
    try:
        subprocess.run(  # noqa: S603
            command, cwd=cwd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=_TIME_LIMIT, check=False
        )
    except subprocess.TimeoutExpired:
        return None
    return time.perf_counter() - start


def _time(command: list[str], cwd: Path) -> float | None:
    # One run to warm up the file system cache, then the mean of the others.
    if _run(command, cwd) is None:
        return None
    times = []
    for _ in range(_RUNS):
        seconds = _run(command, cwd)
        if seconds is None:
            return None
        times.append(seconds)
    return sum(times) / len(times)


def _label(seconds: float | None) -> str:
    return f">{_TIME_LIMIT:.0f}s" if seconds is None else f"{seconds:.2f}s"


def _chart(results: dict[str, float | None]) -> str:
    """Draw the results as horizontal bars, scaled to the time limit, in colors that follow the page theme."""
    width, row, name_width, bar_width = 640, 36, 90, 460
    height = row * len(results) + 40
    rows = []
    for index, (name, seconds) in enumerate(results.items()):
        y = 10 + index * row
        length = bar_width * min(seconds if seconds is not None else _TIME_LIMIT, _TIME_LIMIT) / _TIME_LIMIT
        bar_class = "bar main" if name == "unref" else "bar"
        weight = ' font-weight="bold"' if name == "unref" else ""
        rows.append(
            f'<text x="{name_width - 10}" y="{y + 17}" text-anchor="end"{weight}>{name}</text>'
            f'<rect class="{bar_class}" x="{name_width}" y="{y + 2}" width="{max(length, 2):.1f}" height="22" rx="3"/>'
            f'<text x="{name_width + max(length, 2) + 8:.1f}" y="{y + 17}"{weight}>{_label(seconds)}</text>'
        )
    axis_y = height - 22
    ticks = "".join(
        f'<text class="tick" x="{name_width + bar_width * t / _TIME_LIMIT:.1f}" y="{axis_y + 16}" '
        f'text-anchor="middle">{t:.0f}s</text>'
        for t in range(0, int(_TIME_LIMIT) + 1, 10)
    )
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" font-family="system-ui, -apple-system, sans-serif" font-size="14">'
        "<style>"
        "text{fill:#1f2328}.tick{fill:#656d76;font-size:12px}.bar{fill:#d0d7de}.bar.main{fill:#e8a33d}"
        ".axis{stroke:#d0d7de}"
        "@media (prefers-color-scheme: dark){text{fill:#e6edf3}.tick{fill:#9198a1}.bar{fill:#3d444d}"
        ".axis{stroke:#3d444d}}"
        "</style>"
        f'<line class="axis" x1="{name_width}" y1="{axis_y}" x2="{name_width + bar_width}" y2="{axis_y}"/>'
        f"{''.join(rows)}{ticks}</svg>\n"
    )


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        cpython = _download_cpython(Path(directory) / "source")
        commands = {"unref": [str(Path(sys.executable).parent / "unref")]}
        for name, (requirement, arguments) in _TOOLS.items():
            executable = _install(name, requirement, Path(directory) / name)
            commands[name] = [str(executable), *arguments]

        results = {}
        for name, command in commands.items():
            print(f"Timing {name}...")
            results[name] = _time(command, cpython)

    print(f"\n| Tool | Time to check CPython {_CPYTHON_TAG} |")
    print("| --- | --- |")
    for name, seconds in results.items():
        print(f"| {name} | {_label(seconds)} |")

    _CHART.parent.mkdir(exist_ok=True)
    _CHART.write_text(_chart(results))
    print(f"\nWrote {_CHART}")


if __name__ == "__main__":
    main()
