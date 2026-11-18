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

Defaults to the active virtual environment, or else the `.venv` of the project root. An environment that
runs `unref` itself, such as that of a pre-commit or prek hook, is skipped.

### `dummy-variable-rgx`

Names of variables that are unused on purpose, where Python needs a name to bind, such as an unpacking: `key, _rest = pair()`. Same option and default as
[Ruff's](https://docs.astral.sh/ruff/settings/#lint_dummy-variable-rgx), which matches `_`, `__` and
`_rest`.

```toml
[tool.unref]
dummy-variable-rgx = "^_$|^unused_"
```

### `frameworks`

Frameworks whose built-in rules apply: what a framework finds and calls by itself counts as used. A
framework that is not listed keeps its default. In a file that is not `pyproject.toml`, the table is
`[frameworks]`.

```toml
[tool.unref.frameworks]
unittest = false
```

#### `pytest`

Enabled by default: yes.

- In `test_*.py` and `*_test.py` files: `test*` functions, `Test*` classes and their `test*` methods,
  setup and teardown functions.
- Anywhere: `pytest_*` hooks and fixtures.

A project that sets `python_files`, `python_classes` or `python_functions` adds its own names with an
[override](#overrides):

```toml
[[tool.unref.overrides]]
paths = ["tests"]
used-names = ["check_*", "Describe*"]
```

#### `unittest`

Enabled by default: yes.

- Subclasses of `unittest.TestCase`, with their `test*`, `setUp` and `tearDown` methods.

#### `pydantic`

Enabled by default: yes.

- In subclasses of `pydantic.BaseModel`: fields and `model_config`.
- Anywhere: methods with Pydantic's validator and serializer decorators, and `computed_field`.

#### `dataclasses`

Enabled by default: yes.

- Fields of classes with `dataclasses.dataclass`, Pydantic's `dataclass` or an attrs decorator such as
  `attrs.define`.
- Fields of `NamedTuple` subclasses.

#### `sqlalchemy`

Enabled by default: yes.

- In declarative models and their mixins: attributes annotated `Mapped`, or set to `mapped_column`,
  `Column`, `relationship` and the like.
- Anywhere: functions with `event.listens_for`, `validates`, `reconstructor` or `declared_attr`.

#### `fastapi`

Enabled by default: yes.

- Functions and methods with a route or event decorator, such as `app.get` or `router.websocket`, of a
  `FastAPI()` or `APIRouter()`, or an instance of a subclass, set at module level or in the same function.

#### `flask`

Enabled by default: yes.

- Functions and methods with a route, error or request decorator, such as `app.route` or
  `bp.errorhandler`, of `Flask()` or `Blueprint()`, or an instance of a subclass, set at module level or
  in the same function, of a parameter annotated as one, of a module-level variable set to what a
  factory makes, or of `current_app`.
- Commands of `cli.command` and `cli.group` of these, and of `command` and `group` of an `AppGroup()`
  or of such a group.
- Module-level `create_app` and `make_app` functions that make a `Flask()`, and module-level `app` and
  `application` variables set to `Flask()` or to what such a factory makes.
- HTTP method handlers of `MethodView` subclasses, and `dispatch_request` of `View` subclasses.

#### `sphinx`

Enabled by default: yes.

- In Sphinx's configuration files: module-level variables not starting with `_`, and the `setup` and
  `linkcode_resolve` functions.

The configuration files are each `conf.py` that sets `project`, `extensions` or `html_theme`, or those that
`configuration` names. A table turns the rules on:

```toml
[tool.unref.frameworks.sphinx]
configuration = ["docs/conf.py"]
```

#### `django`

Enabled by default: yes.

- In settings modules: module-level uppercase names, such as `DEBUG`.
- In the module that `ROOT_URLCONF` names: `urlpatterns` and `handler400`, `handler403`, `handler404`,
  `handler500`.
- In modules that `include("...")` names: `urlpatterns` and `app_name`.
- Module-level variables set to `get_wsgi_application()` or `get_asgi_application()`.
- Subclasses of `AppConfig`, with their `ready` method and attributes such as `name`.

The settings modules are those that `os.environ.setdefault("DJANGO_SETTINGS_MODULE", ...)` names, those
that `settings` names, and the modules they star import. A table turns the rules on:

```toml
[tool.unref.frameworks.django]
settings = ["config/settings/production.py"]
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
