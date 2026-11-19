# Configuration

`unref` reads the `[tool.unref]` table of the closest `pyproject.toml`, from the current directory up:

```toml
[tool.unref]
used-decorators = ["app.command", "app.route"]
exclude = ["generated"]
```

As with ty, the project root is the directory of the closest `pyproject.toml` with a `[tool.unref]`
table, or else of the closest `pyproject.toml`. Its paths are relative to the project root, which is
checked by default.

Another file can be given with `--config`. In a file not named `pyproject.toml`, the options are at the
top level. Its paths are relative to the current directory, as in Ruff.

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
| `show-transitive`    | Also print what only other unused definitions reach.              |
| `overrides`          | Options for some of the definitions, see [Overrides](#overrides). |

### `used-names`

Names that count as used, such as those a framework looks up. Patterns, with `*`, `?` and `[abc]`.

```toml
[tool.unref]
used-names = ["handle_*", "visit_*"]
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
`.gitignore`, anchored at the project root, as [ty's `exclude`](https://docs.astral.sh/ty/exclusions/):

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

Defaults to the first of these that exists, in this order:

1. The active virtual environment, unless it is the one that `unref` is installed in, such as that of a
   pre-commit or prek hook.
2. The `.venv` of the project root.
3. The virtual environment that `unref` is installed in.

### `dummy-variable-rgx`

Names of variables that are unused on purpose, where Python needs a name to bind, such as an unpacking: `key, _rest = pair()`. Same option and default as
[Ruff's](https://docs.astral.sh/ruff/settings/#lint_dummy-variable-rgx), which matches `_`, `__` and
`_rest`.

```toml
[tool.unref]
dummy-variable-rgx = "^_$|^unused_"
```

### `frameworks`

Frameworks whose built-in rules apply, see [Frameworks](frameworks.md). A framework that is not listed
keeps its default. In a file that is not `pyproject.toml`, the table is `[frameworks]`.

```toml
[tool.unref.frameworks]
unittest = false
```

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

### `show-transitive`

By default, only the top of each unused chain is printed: a definition that only other unused
definitions call, read or contain is left out. Definitions that only reach each other, such as two
functions that call each other, are all printed. Set it to print every finding, as
[Ruff's `show-fixes`](https://docs.astral.sh/ruff/settings/#show-fixes) prints every fix. The exit code
is the same either way.

```toml
[tool.unref]
show-transitive = true
```

## Command line

`unref [paths]` checks the given files and directories, or the project root.

| Option            | Description                                                          |
| ----------------- | -------------------------------------------------------------------- |
| `--config`        | Configuration file. Defaults to the closest `pyproject.toml`.        |
| `--python`        | The [`python`](#python) option. Overrides the configuration.         |
| `--output-format` | The [`output-format`](#output-format) option. Overrides the configuration. |
| `--show-transitive`, `--no-show-transitive` | The [`show-transitive`](#show-transitive) option. Overrides the configuration. |
| `--jobs`, `-j`    | Number of threads. Defaults to one for each CPU.                     |
| `--version`       | Print the version and exit.                                          |

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

Options that apply only to some definitions. An override selects them with at least one selector:

| Selector           | Selects                                                                                |
| ------------------ | -------------------------------------------------------------------------------------- |
| `paths`            | Definitions in these files and directories.                                            |
| `base-classes`     | Definitions in subclasses of these classes, such as `ast.NodeVisitor`.                 |
| `class-decorators` | Definitions in classes with these decorators, such as `plugins.register`.              |

With several selectors, an override applies to what matches all of them. It then takes these options,
which add to those of the project:

| Option            | Description                                                  |
| ----------------- | ------------------------------------------------------------ |
| `used-names`      | As [`used-names`](#used-names), for the selected definitions.      |
| `used-decorators` | As [`used-decorators`](#used-decorators), for the selected definitions. |

```toml
[[tool.unref.overrides]]
paths = ["migrations"]
used-names = ["Migration"]

[[tool.unref.overrides]]
base-classes = ["ast.NodeVisitor"]
used-names = ["visit_*"]

[[tool.unref.overrides]]
class-decorators = ["plugins.register"]
used-names = ["*"]
```

A base class or decorator is its dotted name as the project imports it. Subclasses of subclasses count
too, including through classes from outside the project. In a file that is not `pyproject.toml`, the
table is `[[overrides]]`.
