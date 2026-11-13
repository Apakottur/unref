# unref

Find unreferenced Python code.

`unref` reports the unused functions, classes, methods, variables and attributes of a Python project:
those that nothing references.

## Quick start

Run it with [`uvx`](https://docs.astral.sh/uv/):

```shell
$ uvx unref
src/project/models.py:12:7: unused-class 'Account'
src/project/utils.py:40:5: unused-function 'parse_date'
```

Or as a [pre-commit](https://pre-commit.com/) or [prek](https://prek.j178.dev/) hook, alongside
[Ruff](https://docs.astral.sh/ruff/):

```yaml
repos:
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.17.4
    hooks:
      - id: ruff-check
      - id: ruff-format
  - repo: https://github.com/Apakottur/unref
    rev: "" # The latest release.
    hooks:
      - id: unref
```

## Usage

### Command line

`unref` checks the project in the current directory, or only the files and directories it is given:

```shell
uvx unref src scripts
```

`unref` exits with a non-zero status if anything is found. See the [rules](rules.md) for what it reports
and how to ignore a finding, and the [command line](configuration.md#command-line) for its options.

### Python

```python
import unref

for finding in unref.check(["src"]):
    print(finding)
```

See the [Python API](reference/api.md).

### pre-commit and prek

The hook in the [quick start](#quick-start) runs next to Ruff, which covers the unused imports and local
variables that `unref` leaves out.

Set `rev` to the [latest release](https://github.com/Apakottur/unref/releases), or let `pre-commit autoupdate`
or `prek autoupdate` fill it in.

The hook checks the whole project, not only the changed files: a definition is unused only if no file
references it.

### GitHub Actions

With `--output-format github`, the findings show as annotations on the files of a pull request:

```yaml
jobs:
  unref:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: astral-sh/setup-uv@v10
      - run: uv sync # Optional: lets unref follow base classes from the dependencies, see `python`.
      - run: uvx unref --output-format github
```

## Why

Unused code piles up in every codebase, faster than ever now that AI tools write much of it.

[Ruff](https://docs.astral.sh/ruff/) does not find unused code yet, see
[astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872). `unref` fills that gap until it does:

- **Just works**: sensible defaults for common frameworks, so most projects need little or no
  configuration.
- **Made for Ruff**: same conventions, options and output style, and it covers what Ruff leaves out.
- **Fast**: checks all of CPython in seconds.
- **Modern**: ty-style rule codes, configuration in `pyproject.toml`, and a typed Python API.

The name is short for "unreferenced".
