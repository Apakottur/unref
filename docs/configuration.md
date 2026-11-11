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
