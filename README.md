[![image](https://img.shields.io/pypi/v/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/pypi/l/unref.svg)](https://github.com/Apakottur/unref/blob/main/LICENSE)
[![image](https://img.shields.io/pypi/pyversions/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/readthedocs/unref.svg)](https://unref.readthedocs.io/)

# unref

Find dead code in Python projects: the unused functions, classes, methods, variables and attributes that
nothing references.

**Documentation**: [unref.readthedocs.io](https://unref.readthedocs.io/)

## Why

Unused code piles up in every codebase, faster than ever now that AI tools write much of it.

[Ruff](https://docs.astral.sh/ruff/) does not find unused code yet, see
[astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872). `unref` fills that gap until it does:

- **Just works**: built-in rules for pytest, unittest, Pydantic, SQLAlchemy and FastAPI, so most projects need
  little or no configuration.
- **Ruff-style**: same conventions, options and output style, and it covers what Ruff leaves out.
- **Fast**: written in Rust on Ruff's parser, it checks all of CPython in under a second.
- **Modern**: ty-style rule codes and configuration in `pyproject.toml`.

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
    rev: v0.1.0 # Or run `pre-commit autoupdate`.
    hooks:
      - id: unref
```

## How it works

- **Matched by module**: a function, class or variable is used only when its own module reads it, or
  another module reaches it through an import, `module.name` or `__all__`.
- **Installed base classes**: base classes from the project's environment are followed, so the methods a
  subclass overrides count as used.
- **Ignore comments by code**: `# unref: ignore[...]` names the rule it ignores, and a comment that ignores
  nothing is reported.

## Benchmark

Checking the whole [CPython](https://github.com/python/cpython) codebase (v3.14.0, about 2,100 files):

| Tool                                               | Time   |
| -------------------------------------------------- | ------ |
| unref                                              | 0.94s  |
| unref (one thread)                                 | 1.59s  |
| unref 0.1.1, the last release written in Python    | 5.40s  |
| [vulture](https://github.com/jendrikseipp/vulture) | 19.68s |

Measured with [`scripts/benchmark.py`](https://github.com/Apakottur/unref/blob/main/scripts/benchmark.py).

## Contributing

False positives and other bugs are welcome as [issues](https://github.com/Apakottur/unref/issues). To
contribute code, see [CONTRIBUTING.md](https://github.com/Apakottur/unref/blob/main/CONTRIBUTING.md).
