#!/usr/bin/env python
"""
Wait for a release of unref on PyPI, then for its mirror in unref-pre-commit, starting the mirror until it has one.

Run with `uv run --only-group dev scripts/mirror_release.py VERSION`, with a version such as `1.2.3`. The environment
holds two tokens: `GITHUB_TOKEN`, to read the tags of unref-pre-commit, and `MIRROR_TOKEN`, of the GitHub App that
can start the workflows of unref-pre-commit.

The mirror finds the release through PyPI's JSON API, which is cached and can name it later than this script saw it.
A mirror that finds no release succeeds without a tag, so the mirror is started again whenever none is running and
there is still no tag. The mirror itself retries `uv lock` for up to 12 minutes, until PyPI's simple index names the
release, so the wait for the tag is longer than that.
"""

import os
import time
from typing import Annotated

import httpx
import typer

_MIRROR_API = "https://api.github.com/repos/Apakottur/unref-pre-commit"
_MIRROR_WORKFLOW = "mirror.yml"
_PYPI_DELAY_SECONDS = 10
_PYPI_ATTEMPTS = 60
_MIRROR_DELAY_SECONDS = 60
_MIRROR_ATTEMPTS = 25

app = typer.Typer(add_completion=False)


def _log(message: str) -> None:
    # The output of a step is a pipe, which Python would otherwise buffer until the end of the step.
    print(message, flush=True)


def _github(token: str) -> httpx.Client:
    return httpx.Client(
        base_url=_MIRROR_API,
        headers={"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json"},
        timeout=30,
    )


def _wait_for_pypi(version: str) -> None:
    for attempt in range(1, _PYPI_ATTEMPTS + 1):
        response = httpx.get(f"https://pypi.org/pypi/unref/{version}/json", timeout=30)
        if response.status_code == httpx.codes.OK:
            _log(f"PyPI has unref {version}.")
            return
        _log(f"Attempt {attempt} of {_PYPI_ATTEMPTS}: PyPI has no unref {version} yet ({response.status_code}).")
        time.sleep(_PYPI_DELAY_SECONDS)
    _log(f"PyPI has no unref {version} after {_PYPI_ATTEMPTS} attempts.")
    raise typer.Exit(1)


def _has_tag(github: httpx.Client, tag: str) -> bool:
    response = github.get(f"/git/ref/tags/{tag}")
    if response.status_code == httpx.codes.NOT_FOUND:
        return False
    response.raise_for_status()
    return True


def _mirror_is_running(mirror: httpx.Client) -> bool:
    response = mirror.get(f"/actions/workflows/{_MIRROR_WORKFLOW}/runs", params={"per_page": 20})
    response.raise_for_status()
    return any(run["status"] != "completed" for run in response.json()["workflow_runs"])


def _start_mirror(mirror: httpx.Client) -> None:
    mirror.post(f"/actions/workflows/{_MIRROR_WORKFLOW}/dispatches", json={"ref": "main"}).raise_for_status()


def _wait_for_mirror(tag: str, github: httpx.Client, mirror: httpx.Client) -> None:
    for attempt in range(1, _MIRROR_ATTEMPTS + 1):
        if _has_tag(github, tag):
            _log(f"unref-pre-commit has {tag}.")
            return
        if _mirror_is_running(mirror):
            _log(f"Attempt {attempt} of {_MIRROR_ATTEMPTS}: unref-pre-commit has no {tag} yet, the mirror is running.")
        else:
            _log(f"Attempt {attempt} of {_MIRROR_ATTEMPTS}: unref-pre-commit has no {tag}, starting the mirror.")
            _start_mirror(mirror)
        time.sleep(_MIRROR_DELAY_SECONDS)
    _log(f"unref-pre-commit has no {tag} after {_MIRROR_ATTEMPTS} attempts.")
    raise typer.Exit(1)


@app.command()
def main(version: Annotated[str, typer.Argument(help="The version of the release, such as 1.2.3.")]) -> None:
    """Wait for a release on PyPI, then for its mirror in unref-pre-commit."""
    _wait_for_pypi(version)
    with _github(os.environ["GITHUB_TOKEN"]) as github, _github(os.environ["MIRROR_TOKEN"]) as mirror:
        _wait_for_mirror(f"v{version}", github, mirror)


if __name__ == "__main__":
    app()
