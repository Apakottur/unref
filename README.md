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

- **Just works**: built-in rules for pytest, unittest, Pydantic and SQLAlchemy, so most projects need little
  or no configuration.
- **Ruff-style**: same conventions, options and output style, and it covers what Ruff leaves out.
- **Fast**: checks all of CPython in seconds.
- **Modern**: ty-style rule codes and configuration in `pyproject.toml`.

## Quick start

Run it with [`uvx`](https://docs.astral.sh/uv/):

```shell
$ uvx unref
src/project/models.py:12:7: unused-class 'Account'
src/project/utils.py:40:5: unused-function 'parse_date'
```

Code that is used in a way `unref` cannot see, such as by a framework, can be ignored with a comment or
[configured](https://unref.readthedocs.io/en/latest/configuration/) once for the project:

```python
def parse_date(text):  # unref: ignore[unused-function]
    ...
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

## Compared to vulture

|                                      | unref                                                                     | [vulture](https://github.com/jendrikseipp/vulture)  |
| ------------------------------------ | ------------------------------------------------------------------------- | --------------------------------------------------- |
| Functions, classes, variables        | Matched to their module, through imports, `module.name` and `__all__`.    | A name read anywhere counts every definition of it. |
| Frameworks                           | Built-in rules for pytest, unittest, Pydantic and SQLAlchemy.             | Whitelist files and ignore options.                 |
| Base classes from installed packages | Read from the project's environment, so overridden methods count as used. | Not followed.                                       |
| Ignore comments                      | By rule code, and the ones that ignore nothing are reported.              | Whitelist files instead.                            |

Checking the whole [CPython](https://github.com/python/cpython) codebase (v3.14.0, about 2,100 files),
measured with [`scripts/benchmark.py`](https://github.com/Apakottur/unref/blob/main/scripts/benchmark.py):

| Tool                | Time   |
| ------------------- | ------ |
| unref               | 3.96s  |
| unref (one process) | 9.32s  |
| vulture             | 14.87s |

## Feedback

`unref` is young. False positives, and anything else it gets wrong, are very welcome as
[issues](https://github.com/Apakottur/unref/issues).

## Contributing

See [CONTRIBUTING.md](https://github.com/Apakottur/unref/blob/main/CONTRIBUTING.md).
