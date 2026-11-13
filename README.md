[![image](https://img.shields.io/pypi/v/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/pypi/l/unref.svg)](https://github.com/Apakottur/unref/blob/main/LICENSE)
[![image](https://img.shields.io/pypi/pyversions/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/readthedocs/unref.svg)](https://unref.readthedocs.io/)

# unref

Find unreferenced Python code.

`unref` reports the unused functions, classes, methods, variables and attributes of a Python project:
those that nothing references.

**Documentation**: [unref.readthedocs.io](https://unref.readthedocs.io/)

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

## Benchmark

Checking the whole [CPython](https://github.com/python/cpython) codebase (v3.14.0, about 2,100 files):

| Tool                | Time   |
| ------------------- | ------ |
| unref               | 3.96s  |
| unref (one process) | 9.32s  |
| vulture             | 14.87s |

Measured with [`scripts/benchmark.py`](https://github.com/Apakottur/unref/blob/main/scripts/benchmark.py).

## Why

Unused code piles up in every codebase, and faster than ever now that AI tools write much of it and
tend to leave unused code behind.

Finding and removing dead code reduces bloat, speeds up AI code scanning and simplifies development.

[Ruff](https://docs.astral.sh/ruff/) does not support this yet, see
[astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872). `unref` exists to fill the gap until
Ruff supports it natively.

`unref` follows [Vulture](https://github.com/jendrikseipp/vulture), the tool that has long filled this
role, as a fast and modern package that follows the conventions of Ruff.

## Contributing

See [CONTRIBUTING.md](https://github.com/Apakottur/unref/blob/main/CONTRIBUTING.md).
