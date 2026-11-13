# Contributing to unref

Thanks for your interest in contributing! To contribute, open a pull request with your changes, or an
issue with your bug or request.

All checks (linters, type checks, and tests) run automatically in CI through GitHub Actions.

## Development environment

Local development is done with [uv](https://docs.astral.sh/uv/getting-started/installation/).

Install all the development dependencies:

```shell
uv sync
```

## Running checks locally

Run the linters with `prek`:

```shell
uv run prek run -a
```

Run the unit tests with `pytest`:

```shell
uv run pytest -c tests/pytest.ini tests
```

### Test cases

The behavior of the check is tested with case projects. Each directory under `tests/cases` is a small
Python project, and `tests/test_cases.py` runs the check on each of them.

A finding is expected with a comment on the line it is reported on, which gives its column and its code,
as the assertions of ty's tests do:

```python
def unused():  # expect: 5 [unused-function]
    return 1
```

Several findings on one line are separated by commas:

```python
first, second = 1, 2  # expect: 1 [unused-variable], 8 [unused-variable]
```

The comment can follow another comment on the line, such as a `# type: ignore[...]`.

The findings must match exactly, so a line without the comment must not have a finding. To test a new
behavior, add a directory with the files that show it.

A case is configured with a `pyproject.toml` in its directory, as a project is, which also gives its
entry points.

### Type checks

Run the type checks with `mypy` and `ty` (both run in CI):

```shell
uv run mypy --config-file linters/mypy.toml .
uv run ty check --project . --config-file linters/ty.toml
```

## Documentation

The documentation site lives in `docs/`, is built with [Zensical](https://zensical.org/), and is hosted on
[Read the Docs](https://unref.readthedocs.io/). The Python API reference is generated from the docstrings.

Preview it locally with live reload:

```shell
uv run --group docs zensical serve
```

Build it the way CI does (warnings, such as broken links, fail the build):

```shell
uv run --group docs zensical build --strict
```

## Releasing

To release a new version, run the interactive script (it requires the [`gh`](https://cli.github.com/) CLI,
authenticated against the repository):

```shell
uv run scripts/release.py
```
