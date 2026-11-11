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

## Configuration

`unref` reads its configuration from the `[tool.unref]` table of the `pyproject.toml` in the current
directory:

```toml
[tool.unref]
used-decorators = ["app.command", "pytest.fixture"]
```

Another file can be given with `--config`. A file that is not named `pyproject.toml` has the options
at the top level:

```toml
used-decorators = ["app.command", "pytest.fixture"]
```

### `used-decorators`

Decorators that make the definitions they decorate count as used. Use it for decorators that register
a function or class, such as a route or a command, which is then never referenced by name.

A decorator is given by its dotted name, as it is written in the code, and covers the decorator with and
without arguments: `app.command` covers `@app.command`, `@app.command()` and `@app.command("name")`.

## Why

Unused code piles up in every codebase, and faster than ever now that AI tools write much of it and
tend to leave unused code behind.

Finding and removing dead code reduces bloat, speeds up AI code scanning and simplifies development.

[Ruff](https://docs.astral.sh/ruff/) does not support this yet, see
[astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872). `unref` exists to fill the gap until
Ruff supports it natively.

## Contributing

See [CONTRIBUTING.md](https://github.com/Apakottur/unref/blob/main/CONTRIBUTING.md).
