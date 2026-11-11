# unref

Find unreferenced Python code.

`unref` reports the functions, classes, methods, variables and attributes of a Python project that
nothing references.

## Installation

unref is available as [`unref`](https://pypi.org/project/unref/) on PyPI. Run it without installing via
[`uvx`](https://docs.astral.sh/uv/), or install it with `uv` (recommended) or `pip`:

```shell
# With uv.
uv tool install unref@latest  # Install unref globally.
uv add --dev unref            # Or add unref to your project.

# With pip.
pip install unref
```

## Usage

### Command line

Check the project in the current directory:

```shell
$ unref
src/project/models.py:12: unused class 'Account'
src/project/utils.py:40: unused function 'parse_date'
```

Or only some files and directories:

```shell
unref src scripts
```

`unref` exits with a non-zero status if anything is found.

### Python

```python
import unref

for finding in unref.check(["src"]):
    print(finding)
```

See the [Python API](reference/api.md).

### pre-commit and prek

`unref` can run as a [pre-commit](https://pre-commit.com/) or [prek](https://prek.j178.dev/) hook, pinned to
the [latest release](https://github.com/Apakottur/unref/releases):

```yaml
repos:
  - repo: https://github.com/Apakottur/unref
    rev: v0.0.1
    hooks:
      - id: unref
```

The hook checks the whole project, not only the changed files: a definition is unused only if no file
references it.

## What is reported

| Finding            | Definition                                                               |
| ------------------ | ------------------------------------------------------------------------ |
| `unused function`  | A function, including a nested one.                                      |
| `unused class`     | A class.                                                                 |
| `unused method`    | A function directly in a class.                                          |
| `unused variable`  | A module-level variable.                                                 |
| `unused attribute` | A class attribute, or an instance attribute (`self.value = ...`).        |

Local variables and imports are not reported, [Ruff](https://docs.astral.sh/ruff/) covers them.

A definition counts as referenced when its name is read or imported anywhere in the project, or listed in
`__all__`. Dunders, such as `__init__` and `__all__`, are never reported.

## Why

Unused code piles up in every codebase, and faster than ever now that AI tools write much of it and
tend to leave unused code behind.

Finding and removing dead code reduces bloat, speeds up AI code scanning and simplifies development.

[Ruff](https://docs.astral.sh/ruff/) does not support this yet, see
[astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872). `unref` exists to fill the gap until
Ruff supports it natively.
