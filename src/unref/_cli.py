import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from unref._api import check


def main(argv: Sequence[str] | None = None) -> int:
    """
    Run the command line interface.

    Args:
        argv: The command line arguments, defaults to the arguments of the current process.

    Returns:
        The exit code: 1 if there are findings, 0 otherwise.
    """
    parser = argparse.ArgumentParser(prog="unref", description="Find unreferenced Python code.")
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        default=[Path()],
        help="files and directories that make up the project (default: the current directory)",
    )
    args = parser.parse_args(argv)

    findings = check(args.paths)
    for finding in findings:
        sys.stdout.write(f"{finding}\n")

    return 1 if findings else 0
