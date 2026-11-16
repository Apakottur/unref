# unref

Find dead code in Python projects.

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
    rev: v0.2.0 # Or run `pre-commit autoupdate`.
    hooks:
      - id: unref
```

## Usage

### Command line

`unref` checks the project that the current directory is in, or only the files and directories it is given:

```shell
uvx unref src scripts
```

`unref` exits with a non-zero status if anything is found. See the [rules](rules.md) for what it reports
and how to ignore a finding, and the [command line](configuration.md#command-line) for its options.

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

- **Just works**: built-in rules for pytest, unittest, Pydantic, dataclasses, SQLAlchemy, FastAPI and Flask, so
  most projects need little or no configuration.
- **Accurate**: names are matched by module, so a used `parse` in one module does not hide an unused `parse`
  in another.
- **Ruff-style**: same conventions, options and output style, ty-style rule codes and configuration in
  `pyproject.toml`.
- **Fast**: written in Rust on Ruff's parser, it checks all of CPython in under a second.

The name is short for "unreferenced".
