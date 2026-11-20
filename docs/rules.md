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
| `invalid-syntax`              | A file with invalid Python syntax or source bytes that are invalid for its encoding. The file is skipped. |
| `unsupported-encoding`        | A file that declares an encoding for which unref has no decoder. The file is skipped. |
| `io-error`                    | A file or directory that cannot be read, such as a given path that does not exist. |

Unused imports and local variables are left to [Ruff](https://docs.astral.sh/ruff/).

A definition counts as used when something references it:

| Definition                 | Used when                                                                                                           |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Function, class, variable  | Read in its module, imported from it, read as `module.name`, listed in its `__all__`, or looked up in its module as `globals()["name"]`. |
| Method, attribute          | Read through its class or a related one (`self.name`, `super().name`, `Class.name`), as `value.name` of any other object, matched by name, or read by its name in its class body. |
| `TypedDict` field          | Read as a key anywhere in the project, matched by name: `value["name"]`, `value.get("name")` or `"name" in value`. Or a value of its TypedDict, made by a call or set to one, is read whole: `{**value}`, `f(**value)`, a loop, `value.items()`, or passed to a function from outside the project, such as `dict(value)` or `json.dumps(value)`. |
| Enum member                | Read as `Color.RED`, or the enum is read whole: `Color(value)`, `list(Color)`, or as the type of a class attribute. |
| Member of an external base | Named like a member of a base class from outside the project, such as `emit` in a subclass of `logging.Handler`.    |
| `visit_*` method           | Always, in a subclass of `ast.NodeVisitor`, whose `visit` calls it by the name of the node.                         |

A base class from outside the project is read from the [environment of the project](configuration.md#python).

`getattr(value, "name")` and `hasattr(value, "name")` read `value.name`. A name that is only known when
the code runs, such as `getattr(value, name)` or `eval("name")`, is not read.

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
