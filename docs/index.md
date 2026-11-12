# unref

Find unreferenced Python code.

`unref` reports the unused functions, classes, methods, variables and attributes of a Python project:
those that nothing references.

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

`unref` can run as a [pre-commit](https://pre-commit.com/) or [prek](https://prek.j178.dev/) hook:

```yaml
repos:
  - repo: https://github.com/Apakottur/unref
    rev: "" # The latest release.
    hooks:
      - id: unref
```

Set `rev` to the [latest release](https://github.com/Apakottur/unref/releases), or let `pre-commit autoupdate`
or `prek autoupdate` fill it in.

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
| `unused comment`   | An `# unref: ignore` comment that ignores nothing.                       |

Local variables and imports are not reported, [Ruff](https://docs.astral.sh/ruff/) covers them.

A definition counts as used when something references it:

- A function, class or variable is read in its own module, imported from it (`from module import name`),
  read through it (`module.name`), or listed in its `__all__`.
- A method or attribute is read as an attribute (`value.name`) anywhere in the project. It is matched by
  name, whatever the object is.

Code in a function runs only when the function is called, so a reference from an unused function does not
count. A function that only calls itself is unused, and so is a function that only an unused function
calls. The rest of a module runs when it is imported, including class bodies, decorators and default
values, so a reference from there always counts.

Dunders, such as `__init__` and `__all__`, are never reported.

## Ignoring a finding

A definition with an `# unref: ignore` comment on its line is not reported:

```python
class Setup:
    def __init__(self, name: str) -> None:
        self.name = name  # unref: ignore
```

The comment goes on the line of the finding, which for a function or class is the line of its `def` or
`class`. For names that a framework uses, prefer the [configuration](configuration.md).

A comment that ignores nothing is reported as an `unused comment`, for example once the definition is
referenced, or covered by the configuration.

## Why

Unused code piles up in every codebase, and faster than ever now that AI tools write much of it and
tend to leave unused code behind.

Finding and removing dead code reduces bloat, speeds up AI code scanning and simplifies development.

[Ruff](https://docs.astral.sh/ruff/) does not support this yet, see
[astral-sh/ruff#872](https://github.com/astral-sh/ruff/issues/872). `unref` exists to fill the gap until
Ruff supports it natively.

The name is short for "unreferenced".
