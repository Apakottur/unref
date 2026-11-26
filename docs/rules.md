# Rules

## What is reported

| Code                          | Finding                                                               |
| ----------------------------- | --------------------------------------------------------------------- |
| `unused-function`             | A function, including a nested one.                                   |
| `unused-class`                | A class.                                                              |
| `unused-method`               | A function directly in a class.                                       |
| `unused-variable`             | A module-level variable.                                              |
| `unused-attribute`            | A class attribute, or an instance attribute (`self.value = ...`).     |
| `unused-ignore-comment`       | An ignore comment, or a code of one, that ignores nothing.            |
| `invalid-ignore-comment`      | An ignore comment that names no code, or that does not parse.         |
| `ignore-comment-unknown-rule` | A code of an ignore comment that unref does not know.                 |
| `invalid-syntax`              | A file that does not parse. It is skipped, the rest is still checked. |
| `io-error`                    | A file or directory that cannot be read, such as a given path that does not exist. |

Unused imports and local variables are left to [Ruff](https://docs.astral.sh/ruff/).

A definition counts as used when something references it:

| Definition                 | Used when                                                                                                           |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Function, class, variable  | Read in its module, imported from it, read as `module.name` (also when the module is [reached as an object](#modules-as-objects)), listed in its `__all__`, looked up in its module as `globals()["name"]`, or defined in a module whose `globals()` is iterated or passed on. |
| Method, attribute          | Read through its class or a related one (`self.name`, `super().name`, `Class.name`), as `value.name` of any other object, matched by name, or read by its name in its class body. A method is also used when it is decorated with an attribute of a name bound before it in its class body, such as `@field.validator`, which registers it on that object. |
| `TypedDict` field          | Read as a key anywhere in the project, matched by name: `value["name"]`, `value.get("name")` or `"name" in value`. Or a value of its TypedDict, made by a call, set to one, or a parameter, variable or attribute of `self` annotated with it, is read whole: `{**value}`, `f(**value)`, a loop, `value.items()`, or passed to a function from outside the project, such as `dict(value)` or `json.dumps(value)`. |
| Enum member                | Read as `Color.RED`, or the enum is read whole: `Color(value)`, `list(Color)`, or as the type of a class attribute or of a parameter of a decorated function, whose decorator can read it. |
| Member of an external base | Named like a member of a base class from outside the project, such as `emit` in a subclass of `logging.Handler`.    |

A base class from outside the project is read from the [environment of the project](configuration.md#python).

`getattr(value, "name")` and `hasattr(value, "name")` read `value.name`. A name that is only known when
the code runs, such as `getattr(value, name)` or `eval("name")`, is not read.

### Modules as objects

A module can be reached as an object rather than by its name, and its names are then read from that
object:

| Module                                                                                         | `module.name` reads `name` in                                       |
| ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `module = import_module("package.module")`, or `__import__` with a `fromlist`                  | `package.module`, as for `import package.module as module`.         |
| `module = import_module(f"plugins.{name}")`, by `+`, `%`, or relative with `package=__package__` | Each module whose name matches the literal parts: here `plugins.*`. |
| `module = import_module(path)`, by a name that has no literal part                             | Each module of the project.                                         |
| A module of the project that is passed on: given to a function, assigned, put in a container, or returned | The module, as for an attribute of any object, matched by name. For `import_module(f"plugins.{name}")`, each module that matches, unless its name has no literal part. |

So in this project, `run`, `setup` and `CONSTANT` are used, and `unused` is not:

```python
# plugins/alpha.py
def run(): ...
def setup(): ...
def unused(): ...


CONSTANT = 5

# main.py
import importlib
from plugins import alpha


def load(name):
    module = importlib.import_module(f"plugins.{name}")
    module.setup()
    return module.run()


def pick(mod):
    return mod.CONSTANT


print(load("alpha"), pick(alpha))
```

A module that is only read by its names, `alpha.run`, is not passed on, and neither is one given to
`setattr`, `monkeypatch.setattr`, `patch.object`, `reload` or `isinstance`. A name of a package that
the package binds to something else, as `from .button import button` does, is not its module. A module
found by a path, such as a migration of Alembic, is not followed.

A reference from inside an unused function does not count, so a function that only an unused function
calls is unused too. By default only the top of each such chain is printed, see
[`show-transitive`](configuration.md#show-transitive). Dunders and `_` are never reported.

## Ignoring a finding

An `# unref: ignore[...]` comment on the line of a finding ignores it, by its code:

```python
class Setup:
    def __init__(self, name: str) -> None:
        self.name = name  # unref: ignore[unused-attribute]
```

Several codes are separated by commas: `# unref: ignore[unused-function, unused-class]`. As with
`# ty: ignore[...]`, the comment must name its codes, and a comment that ignores nothing is reported.

For names that a framework uses, prefer the [configuration](configuration.md).
