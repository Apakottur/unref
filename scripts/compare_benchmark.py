#!/usr/bin/env python
"""
Compare the speed of two unref binaries on the CPython codebase.

Run with `uv run scripts/compare_benchmark.py BASE HEAD REPORT`. It downloads the source of CPython, times both
binaries on it in turns and writes a Markdown report of the change from BASE to HEAD to REPORT. The CI posts this
report on each pull request.
"""

import argparse
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

from benchmark import CPYTHON_TAG, download_cpython

_RUNS = 15
# A slowdown above this fraction on one thread is flagged. Many threads are too noisy on CI runners to flag.
_SLOWDOWN_WARNING = 0.10


def _run(command: list[str], cwd: Path) -> float:
    start = time.perf_counter()
    subprocess.run(command, cwd=cwd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)  # noqa: S603
    return time.perf_counter() - start


def _compare(cpython: Path, base: str, head: str) -> str:
    lines = [
        "## Performance",
        "",
        f"Time to check CPython {CPYTHON_TAG}: the median of {_RUNS} runs of each binary, run in turns.",
        "",
        "| Threads | Base | Head | Change |",
        "| --- | --- | --- | --- |",
    ]
    warning = None
    for name, options in {"One": ["--jobs", "1"], "All": []}.items():
        print(f"Timing with {name.lower()} threads...")
        commands = {"base": [base, *options], "head": [head, *options]}
        times: dict[str, list[float]] = {"base": [], "head": []}
        # One run of each to warm up the file system cache. The binaries then run in turns, so that a slow
        # period of the machine slows both, and swap their order each turn, as the second run of a turn is
        # otherwise a few percent slower.
        for command in commands.values():
            _run(command, cpython)
        for turn in range(_RUNS):
            for binary in ("base", "head") if turn % 2 == 0 else ("head", "base"):
                times[binary].append(_run(commands[binary], cpython))
        base_median, head_median = statistics.median(times["base"]), statistics.median(times["head"])
        change = head_median / base_median - 1
        lines.append(f"| {name} | {base_median:.2f}s | {head_median:.2f}s | {change:+.1%} |")
        if options and change > _SLOWDOWN_WARNING:
            warning = f"unref is {change:.1%} slower on one thread."
    if warning is not None:
        lines += ["", "> [!WARNING]", f"> {warning}"]
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("base", type=Path, help="The unref binary to compare with.")
    parser.add_argument("head", type=Path, help="The unref binary to time.")
    parser.add_argument("report", type=Path, help="The Markdown file to write the report to.")
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as directory:
        cpython = download_cpython(Path(directory) / "source")
        report = _compare(cpython, str(args.base.resolve()), str(args.head.resolve()))
    args.report.write_text(report)
    print(report)


if __name__ == "__main__":
    main()
