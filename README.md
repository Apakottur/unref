[![image](https://img.shields.io/pypi/v/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/pypi/l/unref.svg)](https://github.com/Apakottur/unref/blob/main/LICENSE)
[![image](https://img.shields.io/pypi/pyversions/unref.svg)](https://pypi.python.org/pypi/unref)
[![image](https://img.shields.io/readthedocs/unref.svg)](https://unref.readthedocs.io/)

# unref

An extremely fast Python dead code finder, written in Rust.

<p align="center">
  <img alt="Time to check CPython with unref and other dead code finders" src="https://raw.githubusercontent.com/Apakottur/unref/main/assets/benchmark.svg">
</p>

<p align="center">
  <i>Checking the whole <a href="https://github.com/python/cpython">CPython</a> codebase (v3.15.0, about 2,300 files),
  measured with <a href="https://github.com/Apakottur/unref/blob/main/scripts/benchmark.py"><code>scripts/benchmark.py</code></a>.</i>
</p>

- **Fast**: 20x faster than [vulture](https://github.com/jendrikseipp/vulture), it checks all of CPython in under a
  second.
- **Just works**: built-in rules for pytest, unittest, Pydantic, SQLAlchemy, FastAPI and Flask, so most projects need
  little or no configuration.
- **Accurate**: names are matched by module, so a used `parse` in one module does not hide an unused `parse`
  in another.
- **Ruff-style**: same conventions, options and output style as [Ruff](https://docs.astral.sh/ruff/), and it
  finds the unused code Ruff leaves out.
- **Modern**: ty-style rule codes, configuration in `pyproject.toml`, built on Ruff's parser.
- **Fits your workflow**: runs with `uvx`, as a pre-commit or prek hook, and in GitHub Actions.

**Documentation**: [unref.readthedocs.io](https://unref.readthedocs.io/)

## Getting Started

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

## Contributing

False positives and other bugs are welcome as [issues](https://github.com/Apakottur/unref/issues). To
contribute code, see [CONTRIBUTING.md](https://github.com/Apakottur/unref/blob/main/CONTRIBUTING.md).
