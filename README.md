[![image](https://img.shields.io/pypi/v/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/pypi/l/unref.svg)](https://github.com/Apakottur/unref/blob/main/LICENSE)
[![image](https://img.shields.io/pypi/pyversions/unref.svg)](https://pypi.python.org/pypi/unref)

# unref

Find unreferenced Python code.

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

Unused code piles up in every codebase, and faster than ever now that AI tools write much of it and
tend to leave unused code behind.

Finding and removing dead code reduces bloat, speeds up AI code scanning and simplifies development.

[Ruff](https://docs.astral.sh/ruff/) does not support this yet, see
[astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872). `unref` exists to fill the gap until
Ruff supports it natively.

## Contributing

See [CONTRIBUTING.md](https://github.com/Apakottur/unref/blob/main/CONTRIBUTING.md).
