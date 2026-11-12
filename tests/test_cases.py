"""
Run the check on every case project under `tests/cases` and compare it with the findings the case expects.

A case is a directory holding a small Python project. A finding is expected with a comment on the line
it is reported on, which gives its column and the code of its rule, as the assertions of ty's tests do:

    def unused():  # expect: 5 [unused-function]

Several findings on one line are separated by commas:

    first, second = 1, 2  # expect: 1 [unused-variable], 8 [unused-variable]

The comment can follow another comment on the line:

    unused = 1  # type: ignore[assignment]  # expect: 1 [unused-variable]

The findings must match exactly: a line without the comment must not have a finding.

A case is configured with a `pyproject.toml` in its directory, which also gives its entry points.
"""

import re
import tokenize
from pathlib import Path

import pytest

import unref
from unref._config import load_config

_CASES_DIR = Path(__file__).parent / "cases"
_EXPECT_PREFIX = "# expect:"
_EXPECTED = re.compile(r"\s*(?P<column>\d+) \[(?P<code>[a-z-]+)\]\s*")

# A finding as compared by the tests: file path relative to the case, line, column and code.
_Finding = tuple[str, int, int, str]


def _expected_findings(case: Path) -> list[_Finding]:
    """
    Collect the findings that the `# expect:` comments of a case ask for.
    """
    findings: list[_Finding] = []
    for path in case.rglob("*.py"):
        with tokenize.open(path) as file:
            for token in tokenize.generate_tokens(file.readline):
                if token.type != tokenize.COMMENT or _EXPECT_PREFIX not in token.string:
                    continue
                for expected in token.string.partition(_EXPECT_PREFIX)[2].split(","):
                    match = _EXPECTED.fullmatch(expected)
                    assert match, f"{path}:{token.start[0]}: invalid expectation {expected!r}"
                    findings.append(
                        (path.relative_to(case).as_posix(), token.start[0], int(match["column"]), match["code"])
                    )
    return sorted(findings)


def _actual_findings(case: Path, monkeypatch: pytest.MonkeyPatch) -> list[_Finding]:
    """
    Run the check on a case from its directory, as the paths of its configuration are relative to the
    current directory, and collect its findings.
    """
    monkeypatch.chdir(case)
    config = load_config()

    return sorted(
        (finding.path.as_posix(), finding.line, finding.column, finding.code)
        for finding in unref.check(
            [Path()],
            used_names=config.used_names,
            used_decorators=config.used_decorators,
            exclude=config.exclude,
            overrides=config.overrides,
            python=config.python,
            dummy_variable_rgx=config.dummy_variable_rgx,
            entry_points=config.entry_points,
            frameworks=config.frameworks,
        )
    )


@pytest.mark.parametrize(
    "case",
    sorted(path for path in _CASES_DIR.iterdir() if path.is_dir()),
    ids=lambda case: case.name,
)
def test_case(case: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    assert _actual_findings(case, monkeypatch) == _expected_findings(case)
