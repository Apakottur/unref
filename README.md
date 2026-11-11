[![image](https://img.shields.io/pypi/v/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/pypi/l/unref.svg)](https://github.com/Apakottur/unref/blob/main/LICENSE)
[![image](https://img.shields.io/pypi/pyversions/unref.svg)](https://pypi.python.org/pypi/unref)

# unref

Find unreferenced Python code.

`unref` reports the functions, classes and methods in a Python codebase that nothing references.

> [!NOTE]
> `unref` is in early development and does not do anything useful yet.

## Usage

Check the project in the current directory, using [`uvx`](https://docs.astral.sh/uv/):

```shell
uvx unref
```

Or from Python:

```python
import unref

findings = unref.check(["."])
```

Or as a [pre-commit](https://pre-commit.com/) / [prek](https://prek.j178.dev/) hook, pinned to the
[latest release](https://github.com/Apakottur/unref/releases):

```yaml
repos:
  - repo: https://github.com/Apakottur/unref
    rev: v0.0.1
    hooks:
      - id: unref
```

## Why

Finding dead code needs whole-program analysis: a function is unused only if no file in the project
references it. [Ruff](https://docs.astral.sh/ruff/) analyses one file at a time and does not have this
feature yet, see [astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872).

Use `unref` today, and migrate when Ruff ships it.

## Contributing

See [CONTRIBUTING.md](https://github.com/Apakottur/unref/blob/main/CONTRIBUTING.md).
