#!/usr/bin/env python
"""
Compare the findings of two unref binaries on open source projects, as mypy_primer does for mypy.

Run with `uv run scripts/ecosystem.py BASE HEAD REPORT`. It clones the latest commit of each project, runs both
binaries on it with its own configuration, if any, and writes the findings that HEAD adds or removes as a Markdown
report to REPORT. The CI posts this report on each PR.
"""

import tempfile
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

import shpyx
import typer

# The projects that unref runs on.
_PROJECTS = {
    "black": "https://github.com/psf/black",
    "django": "https://github.com/django/django",
    "fastapi": "https://github.com/fastapi/fastapi",
    "flask": "https://github.com/pallets/flask",
    "freqtrade": "https://github.com/freqtrade/freqtrade",
    "full-stack-fastapi-template": "https://github.com/fastapi/full-stack-fastapi-template",
    "mealie": "https://github.com/mealie-recipes/mealie",
    "openlibrary": "https://github.com/internetarchive/openlibrary",
    "prefect": "https://github.com/PrefectHQ/prefect",
    "pydantic": "https://github.com/pydantic/pydantic",
    "pytest": "https://github.com/pytest-dev/pytest",
    "rich": "https://github.com/Textualize/rich",
    "saleor": "https://github.com/saleor/saleor",
    "sphinx": "https://github.com/sphinx-doc/sphinx",
    "sqlalchemy": "https://github.com/sqlalchemy/sqlalchemy",
    "starlette": "https://github.com/Kludex/starlette",
    "strawberry": "https://github.com/strawberry-graphql/strawberry",
    "werkzeug": "https://github.com/pallets/werkzeug",
    "zulip": "https://github.com/zulip/zulip",
}
# The changes shown for each project. A comment on GitHub holds at most 65536 characters.
_MAX_CHANGES = 50
# The columns of the table of changed projects, after the project.
_COLUMNS = ("Before", "After", "Change")
# The exit code of unref when it fails, rather than finding unused code or not.
_ERROR = 2

app = typer.Typer(add_completion=False)


@dataclass(frozen=True)
class _Run:
    findings: set[str]
    error: str | None


def _clone(url: str, destination: Path) -> str:
    """Clone the latest commit of a project, and return its short SHA."""
    shpyx.run(["git", "clone", "--quiet", "--depth", "1", url, destination])
    return shpyx.run(["git", "rev-parse", "--short", "HEAD"], exec_dir=destination).stdout.strip()


def _run(binary: Path, project: Path) -> _Run:
    result = shpyx.run([binary], exec_dir=project, verify_return_code=False)
    if result.return_code == _ERROR:
        return _Run(set(), result.stderr.strip())
    return _Run(set(result.stdout.splitlines()), None)


def _position(finding: str) -> tuple[str, int, int]:
    """The path, line and column of a finding, which unref prints as `path:line:column: code 'name'`."""
    path, line, column, _ = finding.split(":", 3)
    return path, int(line), int(column)


def _count(run: _Run) -> str:
    """The number of findings of a run, or `failed`."""
    return "failed" if run.error is not None else str(len(run.findings))


def _row(name: str, url: str, sha: str, base: _Run, head: _Run) -> str | None:
    """A row of the table of changed projects, whose name opens its changes, or `None` if it has none."""
    added, removed = head.findings - base.findings, base.findings - head.findings
    if not added and not removed and head.error == base.error:
        return None
    lines = [f'<tr><td><details><summary><a href="{url}/tree/{sha}">{name}</a></summary>', ""]
    if head.error != base.error:
        for label, error in (("Base", base.error), ("Head", head.error)):
            if error is not None:
                lines += [f"{label} failed:", "", "```", error, "```", ""]
    changes = sorted(
        [("-", finding) for finding in removed] + [("+", finding) for finding in added],
        key=lambda change: _position(change[1]),
    )
    if changes:
        lines += ["```diff", *(f"{sign} {finding}" for sign, finding in changes[:_MAX_CHANGES])]
        if len(changes) > _MAX_CHANGES:
            lines.append(f"... and {len(changes) - _MAX_CHANGES} more")
        lines += ["```", ""]
    numbers = (_count(base), _count(head), f"+{len(added)} -{len(removed)}")
    lines.append("</details></td>" + "".join(f'<td align="right">{number}</td>' for number in numbers) + "</tr>")
    return "\n".join(lines)


def _changed(rows: list[str]) -> str:
    """A table of the projects with changes, open."""
    header = "<tr><th>Project</th>" + "".join(f'<th align="right">{name}</th>' for name in _COLUMNS) + "</tr>"
    return "\n".join(
        [
            f"<details open><summary>Changed ({len(rows)})</summary>",
            "",
            "<table>",
            header,
            *rows,
            "</table>",
            "",
            "</details>",
        ]
    )


def _unchanged(rows: list[tuple[str, str, str, _Run]]) -> str:
    """A table of the projects without changes, with their number of findings, folded."""
    lines = [f"<details><summary>Unchanged ({len(rows)})</summary>", "", "| Project | Findings |", "| --- | ---: |"]
    for name, url, sha, run in rows:
        lines.append(f"| [{name}]({url}/tree/{sha}) | {_count(run)} |")
    lines += ["", "</details>"]
    return "\n".join(lines)


@app.command()
def compare(base: Path, head: Path, report: Path) -> None:
    """Run two unref binaries on each project, and write the findings that HEAD adds or removes to REPORT."""
    base, head = base.resolve(), head.resolve()
    with tempfile.TemporaryDirectory() as directory:
        projects = {name: Path(directory) / name for name in _PROJECTS}
        print(f"Cloning {len(_PROJECTS)} projects...")
        with ThreadPoolExecutor() as executor:
            shas = dict(zip(_PROJECTS, executor.map(_clone, _PROJECTS.values(), projects.values()), strict=True))
        changed, unchanged = [], []
        for name, url in _PROJECTS.items():
            print(f"Checking {name}...")
            project = projects[name]
            head_run = _run(head, project)
            row = _row(name, url, shas[name], _run(base, project), head_run)
            if row is None:
                unchanged.append((name, url, shas[name], head_run))
            else:
                changed.append(row)

    summary = f"Findings on {len(_PROJECTS)} projects: {len(changed) or 'none'} changed."
    parts = ["## Ecosystem", summary]
    if changed:
        parts.append(_changed(changed))
    if unchanged:
        parts.append(_unchanged(unchanged))
    text = "\n\n".join(parts) + "\n"
    report.write_text(text)
    print(text)


if __name__ == "__main__":
    app()
