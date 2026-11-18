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

# Projects that use the frameworks that unref knows.
_PROJECTS = {
    "django": "https://github.com/django/django",
    "fastapi": "https://github.com/fastapi/fastapi",
    "flask": "https://github.com/pallets/flask",
    "full-stack-fastapi-template": "https://github.com/fastapi/full-stack-fastapi-template",
    "mealie": "https://github.com/mealie-recipes/mealie",
    "pydantic": "https://github.com/pydantic/pydantic",
    "pytest": "https://github.com/pytest-dev/pytest",
    "rich": "https://github.com/Textualize/rich",
    "saleor": "https://github.com/saleor/saleor",
    "sphinx": "https://github.com/sphinx-doc/sphinx",
    "sqlalchemy": "https://github.com/sqlalchemy/sqlalchemy",
}
# The changes shown for each project. A comment on GitHub holds at most 65536 characters.
_MAX_CHANGES = 50
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


def _section(name: str, url: str, sha: str, base: _Run, head: _Run) -> str | None:
    """The changes of a project, folded, or `None` if it has none."""
    added, removed = head.findings - base.findings, base.findings - head.findings
    if not added and not removed and head.error == base.error:
        return None
    lines = [f'<details><summary><a href="{url}/tree/{sha}">{name}</a>: +{len(added)} -{len(removed)}</summary>', ""]
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
    lines.append("</details>")
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
        sections = []
        for name, url in _PROJECTS.items():
            print(f"Checking {name}...")
            project = projects[name]
            section = _section(name, url, shas[name], _run(base, project), _run(head, project))
            if section is not None:
                sections.append(section)

    summary = f"Findings on {len(_PROJECTS)} projects: {len(sections) or 'none'} changed."
    text = "\n\n".join(["## Ecosystem", summary, *sections]) + "\n"
    report.write_text(text)
    print(text)


if __name__ == "__main__":
    app()
