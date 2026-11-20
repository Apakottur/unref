"""
Run the `unref` command as a user runs it, in a process of its own.
"""

import os
import sysconfig
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path

import shpyx

# The command that the tests run: the one installed in the environment of the tests, or another one,
# such as a debug build, that `UNREF_BINARY` names.
UNREF = Path(
    os.environ.get("UNREF_BINARY")
    or Path(sysconfig.get_path("scripts")) / ("unref.exe" if os.name == "nt" else "unref")
)


@dataclass(frozen=True)
class Result:
    """
    What a run of the command gives.

    Attributes:
        exit_code: The exit status.
        stdout: What it prints to its standard output.
        stderr: What it prints to its standard error.
    """

    exit_code: int
    stdout: str
    stderr: str

    @property
    def output(self) -> str:
        """Everything that it prints."""
        return self.stdout + self.stderr


def run(
    args: Sequence[str],
    *,
    env: Mapping[str, str | None] | None = None,
    cwd: Path | None = None,
    binary: Path = UNREF,
    timeout: float | None = None,
) -> Result:
    """
    Run the command.

    Args:
        args: The arguments of the command.
        env: Environment variables to set for the run, or to remove with None.
        cwd: The directory to run it in, or None for the current directory.
        binary: The executable of the command, which is the one that the tests run by default.
        timeout: The maximum time to wait for the command, in seconds.

    Returns:
        What the run gives.
    """
    result = shpyx.run([binary, *args], env=env, exec_dir=cwd, verify_return_code=False, timeout=timeout)
    return Result(exit_code=result.return_code, stdout=result.stdout, stderr=result.stderr)
