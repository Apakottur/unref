"""
Run the `unref` command as a user runs it, in a process of its own.
"""

import os
import subprocess
import sysconfig
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path

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
) -> Result:
    """
    Run the command.

    Args:
        args: The arguments of the command.
        env: Environment variables to set for the run, or to remove with None.
        cwd: The directory to run it in, or None for the current directory.
        binary: The executable of the command, which is the one that the tests run by default.

    Returns:
        What the run gives.
    """
    environment = dict(os.environ)
    for name, value in (env or {}).items():
        if value is None:
            environment.pop(name, None)
        else:
            environment[name] = value
    completed = subprocess.run(  # noqa: S603
        [binary, *args], capture_output=True, text=True, env=environment, cwd=cwd, check=False
    )
    return Result(exit_code=completed.returncode, stdout=completed.stdout, stderr=completed.stderr)
