[![image](https://img.shields.io/pypi/v/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/pypi/l/unref.svg)](https://github.com/Apakottur/unref/blob/main/LICENSE)
[![image](https://img.shields.io/pypi/pyversions/unref.svg)](https://pypi.python.org/pypi/unref)
[![Documentation](https://readthedocs.org/projects/unref/badge/?version=latest)](https://unref.readthedocs.io/)

# unref

Find unreferenced Python code.

`unref` reports the unused functions, classes, methods, variables and attributes of a Python project:
those that nothing references.

**Documentation**: [unref.readthedocs.io](https://unref.readthedocs.io/)

## Quick start

Check the project in the current directory, using [`uvx`](https://docs.astral.sh/uv/):

```shell
$ uvx unref
src/project/models.py:12: unused class 'Account'
src/project/utils.py:40: unused function 'parse_date'
```

The tests, fixtures and hooks that pytest and unittest find by themselves count as used, without any
configuration. See the [configuration](https://unref.readthedocs.io/en/latest/configuration/) to change
that, and for the other options.

## Why

Unused code piles up in every codebase, and faster than ever now that AI tools write much of it and
tend to leave unused code behind.

Finding and removing dead code reduces bloat, speeds up AI code scanning and simplifies development.

[Ruff](https://docs.astral.sh/ruff/) does not support this yet, see
[astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872). `unref` exists to fill the gap until
Ruff supports it natively.

## Contributing

See [CONTRIBUTING.md](https://github.com/Apakottur/unref/blob/main/CONTRIBUTING.md).
