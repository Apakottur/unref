# Configuration

`unref` reads the `[tool.unref]` table of the `pyproject.toml` in the current directory:

```toml
[tool.unref]
used-decorators = ["app.command", "app.route"]
exclude = ["generated"]
```

Another file can be given with `--config`. In a file not named `pyproject.toml`, the options are at the
top level. Paths are relative to the current directory, as in Ruff. The Python API takes the same options as arguments of
[`unref.check`](reference/api.md#unref.check).

## Options

| Option               | Description                                                       |
| -------------------- | ----------------------------------------------------------------- |
| `used-names`         | Names of the definitions that count as used.                      |
| `used-decorators`    | Decorators that make the definitions they decorate count as used. |
| `exclude`            | Files and directories that are not part of the project.           |
| `python`             | Python environment or interpreter that the project runs in.       |
| `dummy-variable-rgx` | Names of the variables that are unused on purpose.                |
| `frameworks`         | Frameworks whose built-in rules apply.                            |
| `output-format`      | How the findings are printed.                                     |
| `overrides`          | Options for some of the definitions, see [Overrides](#overrides). |

### `used-names`

Names that count as used, such as those a framework looks up. Patterns, with `*`, `?` and `[abc]`.

```toml
[tool.unref]
used-names = ["model_config", "visit_*"]
```

### `used-decorators`

Decorators that make what they decorate count as used, such as a route or a command. A decorator is its
full dotted name as written, without the `@`. Arguments are ignored: `app.command` covers
`@app.command("name")`, but not `@command`.

```toml
[tool.unref]
used-decorators = ["app.command"]
```

### `exclude`

Files and directories that are not checked, and whose references do not count. Patterns in the syntax of
`.gitignore`, anchored at the current directory, as [ty's `exclude`](https://docs.astral.sh/ty/exclusions/):

```toml
[tool.unref]
exclude = ["generated", "scripts/legacy.py", "**/fixtures/"]
```

As with ty, common directories such as `.git`, `.venv`, `dist` and `node_modules` are always left out
first, and so are the files that git ignores. `!pattern` brings one back: `exclude = ["!**/dist/"]`. A
path given to `unref` directly is always checked.

### `python`

The Python environment or interpreter of the project, as [ty's `--python`](https://docs.astral.sh/ty/reference/cli/):
a virtual environment, an interpreter, or a `sys.prefix`. A base class from a package installed there,
such as `pydantic.BaseModel`, is followed through its source, so the members it defines count as used.
Nothing is imported or run.

```toml
[tool.unref]
python = ".venv"
```

Defaults to the active virtual environment, or else `./.venv`. An environment that runs `unref` itself,
such as that of a pre-commit or prek hook, is skipped.

### `dummy-variable-rgx`

Names of variables that are unused on purpose, where Python needs a name to bind: an unpacking or a `for`
loop target, such as `key, _rest = pair()`. Same option and default as
[Ruff's](https://docs.astral.sh/ruff/settings/#lint_dummy-variable-rgx), which matches `_`, `__` and
`_rest`.

```toml
[tool.unref]
dummy-variable-rgx = "^_$|^unused_"
```

### `frameworks`

Frameworks whose built-in rules apply: what a framework finds and calls by itself counts as used.
`frameworks = []` turns them all off.

```toml
[tool.unref]
frameworks = ["pytest", "unittest"]
```

| Framework  | Enabled by default | Counts as used                                                                                                                                                                  |
| ---------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pytest`   | `true`             | In `test_*.py` and `*_test.py` files: `test*` functions, `Test*` classes and their `test*` methods, setup and teardown functions. Anywhere: `pytest_*` hooks and fixtures. |
| `unittest` | `true`             | Subclasses of `unittest.TestCase`, with their `test*`, `setUp` and `tearDown` methods.                                                                                         |

A project with custom pytest naming can add what these miss with an [override](#overrides).

### `output-format`

How the findings are printed, as [Ruff's option](https://docs.astral.sh/ruff/settings/#output-format).

```toml
[tool.unref]
output-format = "github"
```

| Format    | Default | Prints                                                                                                                                                                      |
| --------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `concise` | `true`  | One line for each finding: `src/app.py:40:5: unused-function 'parse_date'`.                                                                                                |
| `github`  | `false` | A [GitHub Actions annotation](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands) for each finding, shown on the file in a pull request. |

## Command line

`unref [paths]` checks the given files and directories, or the current directory.

| Option            | Description                                                          |
| ----------------- | -------------------------------------------------------------------- |
| `--config`        | Configuration file. Defaults to `./pyproject.toml`.                  |
| `--python`        | The [`python`](#python) option. Overrides the configuration.         |
| `--output-format` | The [`output-format`](#output-format) option. Overrides the configuration. |
| `--jobs`, `-j`    | Number of processes. Defaults to one for each CPU.                   |
| `--version`       | Print the version and exit.                                          |

The library uses one process by default. Pass `jobs` to `unref.check` for more.

## Entry points

The objects that the entry points in `pyproject.toml` name count as used, and so does what they use:

```toml
[project.scripts]
tool = "tool.cli:main"

[project.entry-points."tool.plugins"]
plugin = "tool.plugins:Plugin.load"
```

Here, `main`, `Plugin` and `Plugin.load` count as used.

## Overrides

Options that apply only to some definitions: those under some paths, in subclasses of some base classes,
or in classes with some decorators. In a file that is not `pyproject.toml`, the table is
`[[overrides]]`.

```toml
[[tool.unref.overrides]]
paths = ["migrations"]
used-names = ["Migration"]

[[tool.unref.overrides]]
base-classes = ["pydantic.BaseModel"]
used-names = ["model_config"]

[[tool.unref.overrides]]
class-decorators = ["dataclasses.dataclass"]
used-names = ["*"]
```

- An override takes `used-names` and `used-decorators`, which add to those of the project.
- With several selectors, an override applies to what matches all of them.
- A base class or decorator is its dotted name as the project imports it, such as `pydantic.BaseModel`.
  Subclasses count, including through classes from outside the project.
