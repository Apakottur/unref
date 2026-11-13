"""
Check a project with several processes.
"""

import shutil
from pathlib import Path

import pytest

import unref
from unref._discover import discover
from unref._files import MIN_PARALLEL_BYTES

_CASES_DIR = Path(__file__).parent / "cases"


def _large_project(directory: Path) -> Path:
    """
    Make a project large enough to be analyzed by several processes, from copies of the test cases.
    """
    # Only the files that the check finds count, not those it leaves out, such as the files in the
    # `.venv` of a case or those that a `.gitignore` of a case ignores.
    size = sum(path.stat().st_size for path in discover([_CASES_DIR], exclude=[]))
    for index in range(MIN_PARALLEL_BYTES // size + 1):
        shutil.copytree(_CASES_DIR, directory / f"copy_{index}")
    return directory


def test_several_processes(tmp_path: Path) -> None:
    project = _large_project(tmp_path)
    assert unref.check([project], jobs=None) == unref.check([project], jobs=1)


@pytest.mark.parametrize("jobs", [0, -1])
def test_invalid_jobs(jobs: int) -> None:
    with pytest.raises(ValueError, match=f"The number of jobs must be at least 1: {jobs}"):
        unref.check([_CASES_DIR / "function"], jobs=jobs)
