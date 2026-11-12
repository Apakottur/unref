# Configuration

`unref` reads its configuration from the `[tool.unref]` table of the `pyproject.toml` in the current
directory:

```toml
[tool.unref]
used-decorators = ["app.command", "pytest.fixture"]
exclude = ["generated"]
```

Another file can be given with `--config`. A file that is not named `pyproject.toml` has the options at
the top level:

```toml
used-decorators = ["app.command", "pytest.fixture"]
exclude = ["generated"]
```

As with ruff, the paths in the configuration, such as `exclude` and `python`, are relative to the
current directory, not to the configuration file.

The Python API takes the same options as arguments of [`unref.check`](reference/api.md#unref.check), and
does not read a configuration file.

## Options

| Option               | Library argument     | Description                                                         |
| -------------------- | -------------------- | ------------------------------------------------------------------- |
| `used-names`         | `used_names`         | Names of the definitions that count as used.                        |
| `used-decorators`    | `used_decorators`    | Decorators that make the definitions they decorate count as used.   |
| `exclude`            | `exclude`            | Files and directories that are not part of the project.             |
| `python`             | `python`             | Virtual environment that the project runs in.                       |
| `dummy-variable-rgx` | `dummy_variable_rgx` | Names of the variables that are unused on purpose, in an unpacking. |

### `used-names`

Names of the definitions that count as used. Use it for names that a framework looks up, such as the
`visit_*` methods of a visitor or the `model_config` of a Pydantic model.

```toml
[tool.unref]
used-names = ["model_config", "visit_*"]
```

A name is a pattern, matched with its case: `*` stands for any text, `?` for one character, and
`[abc]` for one of the characters.

### `used-decorators`

Decorators that make the definitions they decorate count as used. Use it for decorators that register a
function or class, such as a route or a command, which is then never referenced by name.

```toml
[tool.unref]
used-decorators = ["app.command"]
```

```python
@app.command()
def main():  # Not reported.
    ...
```

A decorator is given by its dotted name, as it is written in the code, without the `@`:

- Arguments are ignored: `app.command` covers `@app.command`, `@app.command()` and
  `@app.command("name")`.
- The whole dotted name has to match: `app.command` does not cover `@command` or
  `@registry.app.command`.
- With several decorators on a definition, one match is enough.

### `exclude`

Files and directories that are not part of the project, such as generated code. They are not checked, and
a reference from them does not count.

```toml
[tool.unref]
exclude = ["generated", "scripts/legacy.py", "**/fixtures/"]
```

As with [ty's `exclude`](https://docs.astral.sh/ty/exclusions/), the entries are patterns in the syntax
of `.gitignore`, anchored at the current directory:

- `generated` matches the file or directory `generated` there, not one deeper down. `**/generated`
  matches one at any depth.
- `generated/` matches only a directory.
- `*` matches any text but `/`, `?` one character, and `[abc]` one of the characters.
- `!pattern` brings back what an earlier pattern left out. The last pattern that matches a path decides.

These patterns are always left out first, as ty does, and `exclude` comes after them:

```toml
exclude = [
  "**/.bzr/", "**/.direnv/", "**/.eggs/", "**/.git/", "**/.git-rewrite/", "**/.hg/",
  "**/.mypy_cache/", "**/.nox/", "**/.pants.d/", "**/.pytype/", "**/.ruff_cache/", "**/.svn/",
  "**/.tox/", "**/.venv/", "**/__pypackages__/", "**/_build/", "**/buck-out/", "**/dist/",
  "**/node_modules/", "**/venv/",
]
```

So `exclude = ["!**/dist/"]` checks the `dist` directories.

The files and directories that git ignores, by the `.gitignore` files of the git repository, are also
left out. Other hidden directories, such as `.github`, are part of the project.

A file that is given to `unref` directly is always checked, even when it is excluded or ignored, as
with ruff and ty. So is a directory: only the patterns that match the files and directories under it
leave them out, not those that match the directory itself or one of its parents. `unref .venv/src`
checks the files there, but still leaves out a `.venv` under `.venv/src`.

### `python`

Path to the Python environment or interpreter that the project runs in. A base class from a package
installed there, such as `pydantic.BaseModel`, is followed through its source, so that the members it
defines count as used. The modules of the environment are read, never imported, and its interpreter is
not run.

```toml
[tool.unref]
python = ".venv"
```

As with [ty's `--python`](https://docs.astral.sh/ty/reference/cli/), the path can be:

- A virtual environment, such as `.venv`.
- A Python interpreter, such as `.venv/bin/python`.
- A system Python `sys.prefix` directory, such as `/usr`.

The path is relative to the current directory. It can also be given with `--python`.
Without either, the command uses the active virtual environment (`VIRTUAL_ENV`), or else the `.venv` of
the current directory. An active environment that runs `unref` itself, as in a pre-commit or prek hook,
is skipped. The library uses the environment that runs it, unless it is given `python`.

The `site-packages` of this environment, and the directories that its `.pth` files list, replace the
`site-packages` of the environment that runs `unref`.

### `dummy-variable-rgx`

A regular expression for the names of the variables that are unused on purpose. Such a variable is not
reported where Python needs a name to bind, and only there:

- A target of an unpacking: `key, _rest = pair()`, `head, *_tail = items`, or `_first, _second = pair()`.
- A target of a `for` loop or a comprehension: `for _attempt in range(3):`. Unpacked or not, such a target
  is not reported whatever its name.

```python
key, _public_key = make_pair()  # Not reported.
_response = send()  # Reported: the assignment can be dropped, keeping the call.
```

Elsewhere, such as in a plain or annotated assignment, the binding can be dropped, so the variable is
reported. Functions, classes, methods and attributes of an object, such as `self._cache`, are not
covered: there, `_` marks a private name.

```toml
[tool.unref]
dummy-variable-rgx = "^_$|^unused_"
```

The option is [the one of Ruff](https://docs.astral.sh/ruff/settings/#lint_dummy-variable-rgx), with the
same default, `^(_+|(_+[a-zA-Z0-9_]*[a-zA-Z0-9]+?))$`: `_`, `__`, `_rest` and `_rest_2` match, `_rest_`
does not. As in Ruff, the expression is searched for in the name, so anchor it with `^` and `$`. It uses
the syntax of Python's [`re`](https://docs.python.org/3/library/re.html) module.

`_` itself is never reported, whatever the expression.

## Entry points

The objects that the entry points of the project name count as used, and so does what they use. They
are read from the `[project]` table of the configuration file, when it is a `pyproject.toml`, as
[the entry points specification](https://packaging.python.org/en/latest/specifications/entry-points/)
gives them:

```toml
[project.scripts]
tool = "tool.cli:main"

[project.gui-scripts]
tool-gui = "tool.gui:start"

[project.entry-points."tool.plugins"]
plugin = "tool.plugins:Plugin.load"
```

Here, `main` in `tool/cli.py`, `start` in `tool/gui.py`, and the class `Plugin` with its method `load` in
`tool/plugins.py` count as used.

- The module is found among the files of the project the way an import is.
- With a dotted object, such as `Plugin.load`, each name counts as used where it is found: the class
  `Plugin` alone if it has no `load`.
- The extras of an entry point, as in `"tool.gui:start [gui]"`, are ignored.
- An entry point that names only a module, such as `"tool.plugins"`, makes nothing used.

No other file is read: with `--config unref.toml`, there are no entry points. The library takes them as
the `entry_points` argument of [`unref.check`](reference/api.md#unref.check).

## Overrides

Options that apply only to some of the definitions of the project are given in `overrides` tables. An
override selects by path, by base class, by class decorator, or by several of them.

In a file that is not named `pyproject.toml`, the table is `[[overrides]]`.

### By path

```toml
[tool.unref]
used-decorators = ["app.command"]

[[tool.unref.overrides]]
paths = ["tests"]
used-names = ["test_*", "Test*"]
used-decorators = ["pytest.fixture"]
```

Here, a function named `test_connection` counts as used in `tests`, and is reported anywhere else.

`paths` lists files and directories, relative to the current directory.

### By base class

```toml
[[tool.unref.overrides]]
base-classes = ["pydantic.BaseModel"]
used-names = ["model_config"]

[[tool.unref.overrides]]
base-classes = ["typing.NamedTuple"]
used-names = ["*"]
```

Here, `model_config` counts as used in a Pydantic model, and is reported in any other class. Every field
of a named tuple counts as used.

`base-classes` lists base classes by their dotted name: the module that the class is imported from, and
its name. The override applies to what is defined in a class that inherits from one of them:

- Directly, however the base class is imported: `from pydantic import BaseModel`, `import pydantic`
  or `from pydantic import BaseModel as Base`.
- Through other classes of the project, in any module.
- Through classes from outside the project: with `enum.Enum`, a subclass of `enum.StrEnum` or
  `enum.IntEnum`. Their modules are read from their source, and never imported. They are looked for
  in the directories that the project is imported from, and then in the
  [environment of the project](#python). A module without a Python source, such as a compiled
  module, is not followed.

The name has to be the one that the project imports: `pydantic.BaseModel` does not cover
`from pydantic.main import BaseModel`.

### By class decorator

```toml
[[tool.unref.overrides]]
class-decorators = ["dataclasses.dataclass"]
used-names = ["*"]
```

Here, every field of a data class counts as used.

`class-decorators` lists decorators by their dotted name, without the `@`: the module that the decorator
is imported from, and its name. The override applies to what is defined in a class that has one of them,
however the decorator is imported and with any arguments: `@dataclass`, `@dataclasses.dataclass` or
`@dataclass(frozen=True)`.

A subclass does not take the decorators of its base classes.

### Rules

- An override takes the `used-names` and `used-decorators` options.
- An override needs `paths`, `base-classes` or `class-decorators`. With several of them, it applies to
  what matches all of them: with `paths` and `base-classes`, to the classes under the paths that inherit
  from one of the base classes.
- The options of an override add to those of the project, they do not replace them.
- A definition that several overrides cover gets the options of all of them.
