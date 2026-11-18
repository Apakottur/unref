#!/usr/bin/env python
"""
Time unref on the CPython codebase.

Run with `uv run scripts/benchmark.py COMMAND`. Each command downloads the source of CPython and times on it:

- `tools`: unref and other dead code finders, drawn as a bar chart in `assets/benchmark.svg`. uv builds unref without
  LTO, so first rebuild it with `CARGO_PROFILE_LOCAL_LTO=fat uv sync --reinstall-package unref`.
- `compare BASE HEAD REPORT`: two unref binaries, written as a Markdown report that the CI posts on each PR.
"""

import contextlib
import io
import math
import statistics
import subprocess
import sys
import tarfile
import tempfile
import time
from collections.abc import Generator
from pathlib import Path

import httpx
import shpyx
import typer

_CPYTHON_TAG = "v3.15.0"
_TOOLS = {
    "vulture": ("vulture==2.16", ["."]),
    "deadcode": ("deadcode==2.6.3", ["."]),
    "skylos": ("skylos==5.6.0", [".", "--no-upload", "--no-cache"]),
}
_RUNS = 5
_COMPARE_RUNS = 15
_TIME_LIMIT = 30.0
# A change above this fraction is shown with a warning, red if slower and green if faster, as a big speedup can
# mean that unref stopped doing some of its work. A slowdown above it on a single core fails the comparison, as
# multi core times are too noisy on CI runners to judge.
_CHANGE_LIMIT = 0.10
_CHART = Path(__file__).parent.parent / "assets" / "benchmark.svg"

app = typer.Typer(add_completion=False)


@contextlib.contextmanager
def _cpython() -> Generator[Path]:
    """Download the source of CPython into a temporary directory."""
    url = f"https://codeload.github.com/python/cpython/tar.gz/refs/tags/{_CPYTHON_TAG}"
    print(f"Downloading CPython {_CPYTHON_TAG}...")
    response = httpx.get(url, follow_redirects=True, timeout=120)
    response.raise_for_status()
    with tempfile.TemporaryDirectory() as directory, tarfile.open(fileobj=io.BytesIO(response.content)) as archive:
        archive.extractall(directory, filter="data")
        yield next(Path(directory).iterdir())


def _install(name: str, requirement: str, environment: Path) -> Path:
    print(f"Installing {requirement}...")
    shpyx.run(["uv", "venv", "--quiet", "--python", sys.executable, environment])
    shpyx.run(["uv", "pip", "install", "--quiet", "--python", environment, requirement])
    return environment / "bin" / name


def _run(command: list[str], cwd: Path) -> float:
    """Run a command once and return how long it took, or infinity if it went over the time limit."""
    start = time.perf_counter()
    try:
        subprocess.run(  # noqa: S603
            command, cwd=cwd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=_TIME_LIMIT, check=False
        )
    except subprocess.TimeoutExpired:
        return math.inf
    return time.perf_counter() - start


def _time(commands: list[list[str]], cwd: Path, runs: int) -> list[float]:
    """
    Run the commands in turns and return the median time of each, or infinity for each if a run goes over the
    time limit.

    The first turn only warms up the file system cache. Running in turns makes a slow period of the machine slow
    every command, and the order swaps each turn, as a later run of a turn is otherwise a few percent slower.
    """
    times: list[list[float]] = [[] for _ in commands]
    indices = list(range(len(commands)))
    for turn in range(runs + 1):
        for index in indices if turn % 2 == 0 else indices[::-1]:
            seconds = _run(commands[index], cwd)
            if math.isinf(seconds):
                return [math.inf] * len(commands)
            times[index].append(seconds)
    return [statistics.median(command_times[1:]) for command_times in times]


def _label(seconds: float) -> str:
    return f">{_TIME_LIMIT:.0f}s" if math.isinf(seconds) else f"{seconds:.2f}s"


def _chart(results: dict[str, float]) -> str:
    """Draw the results as horizontal bars, scaled to the time limit, in colors that follow the page theme."""
    width, row, name_width, bar_width = 640, 36, 90, 460
    height = row * len(results) + 40
    rows = []
    for index, (name, seconds) in enumerate(results.items()):
        y = 10 + index * row
        length = max(bar_width * min(seconds, _TIME_LIMIT) / _TIME_LIMIT, 2)
        bar_class = "bar main" if name == "unref" else "bar"
        weight = ' font-weight="bold"' if name == "unref" else ""
        rows.append(
            f'<text x="{name_width - 10}" y="{y + 17}" text-anchor="end"{weight}>{name}</text>'
            f'<rect class="{bar_class}" x="{name_width}" y="{y + 2}" width="{length:.1f}" height="22" rx="3"/>'
            f'<text x="{name_width + length + 8:.1f}" y="{y + 17}"{weight}>{_label(seconds)}</text>'
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


@app.command()
def tools() -> None:
    """Time unref and other dead code finders, and draw the chart."""
    with _cpython() as cpython, tempfile.TemporaryDirectory() as environments:
        commands = {"unref": [str(Path(sys.executable).parent / "unref")]}
        for name, (requirement, arguments) in _TOOLS.items():
            commands[name] = [str(_install(name, requirement, Path(environments) / name)), *arguments]

        results = {}
        for name, command in commands.items():
            print(f"Timing {name}...")
            [results[name]] = _time([command], cpython, _RUNS)

    print(f"\n| Tool | Time to check CPython {_CPYTHON_TAG} |")
    print("| --- | --- |")
    for name, seconds in results.items():
        print(f"| {name} | {_label(seconds)} |")

    _CHART.parent.mkdir(exist_ok=True)
    _CHART.write_text(_chart(results))
    print(f"\nWrote {_CHART}")


@app.command()
def compare(base: Path, head: Path, report: Path) -> None:
    """Time two unref binaries in turns, write the change from BASE to HEAD to REPORT, and fail if HEAD is too slow."""
    rows = []
    changed = False
    slowdown = None
    with _cpython() as cpython:
        for name, options in {"Single core": ["--jobs", "1"], "Multi core": []}.items():
            print(f"Timing {name.lower()}...")
            commands = [[str(base.resolve()), *options], [str(head.resolve()), *options]]
            base_median, head_median = _time(commands, cpython, _COMPARE_RUNS)
            change = head_median / base_median - 1
            cell = f"{change:+.1%}"
            if abs(change) > _CHANGE_LIMIT:
                changed = True
                # GitHub colors text in Markdown only through its math, in which `%` starts a comment. The Markdown
                # turns `\\%` into the `\%` that the math needs.
                escaped = cell.replace("%", r"\\%")
                color = "red" if change > 0 else "green"
                cell = rf"⚠️ $\color{{{color}}}{{{escaped}}}$"
                if options and change > 0:
                    slowdown = change
            rows.append(f"| {name} | {_label(base_median)} | {_label(head_median)} | {cell} |")

    lines = ["## Performance", ""]
    if changed:
        lines += [f"⚠️ **Performance changed significantly**, by more than {_CHANGE_LIMIT:.0%}.", ""]
    lines += [
        f"Time to check CPython {_CPYTHON_TAG}: the median of {_COMPARE_RUNS} runs of each binary, run in turns.",
        "",
        "| Cores | Base | Head | Change |",
        "| --- | --- | --- | --- |",
        *rows,
    ]
    if slowdown is not None:
        lines += [
            "",
            "> [!CAUTION]",
            f"> unref is {slowdown:.1%} slower on a single core, over the limit of {_CHANGE_LIMIT:.0%}.",
        ]
    report.write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    if slowdown is not None:
        raise typer.Exit(1)


if __name__ == "__main__":
    app()
