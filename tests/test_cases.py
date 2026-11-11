"""
Run the check on every case project under `tests/cases` and compare it with the findings the case expects.

A case is a directory holding a small Python project. A finding is expected with a comment on the line
it is reported on:

    def unused():  # expect: unused function

The findings must match exactly: a line without the comment must not have a finding.
"""

import tokenize
from pathlib import Path

import pytest

import unref

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
                (
                    path.relative_to(case).as_posix(),
                    token.start[0],
                    token.string.removeprefix(_EXPECT_PREFIX).strip(),
                )
                for token in tokenize.generate_tokens(file.readline)
                if token.type == tokenize.COMMENT and token.string.startswith(_EXPECT_PREFIX)
            )
    return sorted(findings)


def _actual_findings(case: Path) -> list[_Finding]:
    """
    Run the check on a case and collect its findings.
    """
    return sorted(
        (finding.path.relative_to(case).as_posix(), finding.line, finding.kind) for finding in unref.check([case])
    )


@pytest.mark.parametrize(
    "case",
    sorted(path for path in _CASES_DIR.iterdir() if path.is_dir()),
    ids=lambda case: case.name,
)
def test_case(case: Path) -> None:
    assert _actual_findings(case) == _expected_findings(case)
