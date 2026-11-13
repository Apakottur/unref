"""
Run the check on every case project under `tests/cases` and compare it with the findings the case expects.

A case is a directory holding a small Python project. A finding is expected with a comment on the line
it is reported on, which gives its column and the code of its rule, as the assertions of ty's tests do:

    def unused():  # expect: 5 [unused-function]

Several findings on one line are separated by commas:

    first, second = 1, 2  # expect: 1 [unused-variable], 8 [unused-variable]

The comment can follow another comment on the line:

    unused = 1  # type: ignore[assignment]  # expect: 1 [unused-variable]

A finding that only other unused definitions reach, such as a function that only an unused function
calls or a method of an unused class, is marked `transitive`:

    def unused():  # expect: 5 [unused-function]
        helper()

    def helper():  # expect: 5 [unused-function] transitive

The findings must match exactly: a line without the comment must not have a finding. Each case is run
twice: by default, which prints only the findings that are not transitive, and with `--show-transitive`,
which prints them all.

A case is run through the command, from its directory, so it is configured with a `pyproject.toml` in
its directory, as a project is, which also gives its entry points.
"""

import re
import tokenize
from pathlib import Path

import pytest
from typer.testing import CliRunner

from unref._cli import app

_CASES_DIR = Path(__file__).parent / "cases"
_EXPECT_PREFIX = "# expect:"
_EXPECTED = re.compile(r"\s*(?P<column>\d+) \[(?P<code>[a-z-]+)\](?P<transitive> transitive)?\s*")
# A finding as the command prints it: `path:line:column: code 'name'`.
_FINDING = re.compile(r"(?P<path>.+):(?P<line>\d+):(?P<column>\d+): (?P<code>[a-z-]+) '.*'")

_runner = CliRunner()

# A finding as compared by the tests: file path relative to the case, line, column and code.
_Finding = tuple[str, int, int, str]


def _expected_findings(case: Path, *, show_transitive: bool) -> list[_Finding]:
    """
    Collect the findings that the `# expect:` comments of a case ask for, with those marked `transitive`
    only if they are shown.
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
                    if match["transitive"] and not show_transitive:
                        continue
                    findings.append(
                        (path.relative_to(case).as_posix(), token.start[0], int(match["column"]), match["code"])
                    )
    return sorted(findings)


def _actual_findings(case: Path, monkeypatch: pytest.MonkeyPatch, *, show_transitive: bool) -> list[_Finding]:
    """
    Run the command on a case from its directory, as a user runs it on their project, and collect the
    findings it prints. The paths of the configuration and of the findings are relative to the current
    directory.
    """
    monkeypatch.chdir(case)
    # An environment that is active in the shell that runs the tests is not the one of the case.
    monkeypatch.delenv("VIRTUAL_ENV", raising=False)

    # A single process keeps the check in the process of the tests, which is faster for small cases.
    result = _runner.invoke(app, ["--jobs", "1", "--show-transitive" if show_transitive else "--no-show-transitive"])

    findings: list[_Finding] = []
    for line in result.stdout.splitlines():
        match = _FINDING.fullmatch(line)
        assert match, f"invalid finding {line!r}"
        findings.append((match["path"], int(match["line"]), int(match["column"]), match["code"]))
    # The exit code does not depend on which findings are shown: a transitive finding is reached from
    # another unused definition, so there are findings that are shown exactly when there are findings.
    assert result.exit_code == (1 if findings else 0), result.output
    return sorted(findings)


@pytest.mark.parametrize(
    "case",
    sorted(path for path in _CASES_DIR.iterdir() if path.is_dir()),
    ids=lambda case: case.name,
)
@pytest.mark.parametrize("show_transitive", [False, True], ids=["default", "show-transitive"])
def test_case(case: Path, show_transitive: bool, monkeypatch: pytest.MonkeyPatch) -> None:  # noqa: FBT001
    expected = _expected_findings(case, show_transitive=show_transitive)
    assert _actual_findings(case, monkeypatch, show_transitive=show_transitive) == expected
