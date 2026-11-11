"""
Run the check on every case project under `tests/cases` and compare it with the findings the case expects.

A case is a directory holding a small Python project. A finding is expected with a comment on the line
it is reported on:

    def unused():  # expect: unused function

Several findings on one line are separated by commas:

    first, second = 1, 2  # expect: unused variable, unused variable

The findings must match exactly: a line without the comment must not have a finding.

A case is configured with a `pyproject.toml` in its directory.
"""

import tokenize
from pathlib import Path

import pytest

import unref
from unref._config import load_config

_CASES_DIR = Path(__file__).parent / "cases"
_EXPECT_PREFIX = "# expect:"

# A finding as compared by the tests: file path relative to the case, line number, and kind.
_Finding = tuple[str, int, str]


def _expected_findings(case: Path) -> list[_Finding]:
    """
    Collect the findings that the `# expect:` comments of a case ask for.
    """
    findings: list[_Finding] = []
    for path in case.rglob("*.py"):
        with tokenize.open(path) as file:
            findings.extend(
                (path.relative_to(case).as_posix(), token.start[0], kind.strip())
                for token in tokenize.generate_tokens(file.readline)
                if token.type == tokenize.COMMENT and token.string.startswith(_EXPECT_PREFIX)
                for kind in token.string.removeprefix(_EXPECT_PREFIX).split(",")
            )
    return sorted(findings)


def _actual_findings(case: Path) -> list[_Finding]:
    """
    Run the check on a case, with the configuration of the case, and collect its findings.
    """
    pyproject = case / "pyproject.toml"
    config = load_config(pyproject if pyproject.exists() else None)

    return sorted(
        (finding.path.relative_to(case).as_posix(), finding.line, finding.kind)
        for finding in unref.check([case], used_decorators=config.used_decorators)
    )


@pytest.mark.parametrize(
    "case",
    sorted(path for path in _CASES_DIR.iterdir() if path.is_dir()),
    ids=lambda case: case.name,
)
def test_case(case: Path) -> None:
    assert _actual_findings(case) == _expected_findings(case)
