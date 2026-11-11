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

The Python API takes the same options as arguments of [`unref.check`](reference/api.md#unref.check), and
does not read a configuration file.

## Options

| Option            | Library argument  | Description                                                              |
| ----------------- | ----------------- | ------------------------------------------------------------------------ |
| `used-names`      | `used_names`      | Names of the definitions that count as used.                             |
| `used-decorators` | `used_decorators` | Decorators that make the definitions they decorate count as used.        |
| `exclude`         | `exclude`         | Files and directories that are not part of the project.                  |

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
exclude = ["generated", "scripts/legacy.py"]
```

A path is relative to the directory of the configuration file. Hidden directories, such as `.venv` and
`.git`, are always left out.

## Overrides

Options that apply only to some of the definitions of the project are given in `overrides` tables. An
override selects by path, by base class, or by both.

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

`paths` lists files and directories, relative to the directory of the configuration file.

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

The name has to be the one that the project imports: `pydantic.BaseModel` does not cover
`from pydantic.main import BaseModel`.

### Rules

- An override takes the `used-names` and `used-decorators` options.
- An override needs `paths` or `base-classes`. With both, it applies to the classes under the paths that
  inherit from one of the base classes.
- The options of an override add to those of the project, they do not replace them.
- A definition that several overrides cover gets the options of all of them.
