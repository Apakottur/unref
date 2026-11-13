# Rules

What `unref` reports, with the code of each finding, and how to ignore a finding.

## What is reported

| Code                          | Finding                                                                   |
| ----------------------------- | ------------------------------------------------------------------------- |
| `unused-function`             | A function, including a nested one.                                       |
| `unused-class`                | A class.                                                                  |
| `unused-method`               | A function directly in a class.                                           |
| `unused-variable`             | A module-level variable.                                                  |
| `unused-attribute`            | A class attribute, or an instance attribute (`self.value = ...`).         |
| `unused-ignore-comment`       | An ignore comment, or a code of one, that ignores nothing.                |
| `invalid-ignore-comment`      | An ignore comment that names no code, or that does not parse.             |
| `ignore-comment-unknown-rule` | A code of an ignore comment that unref does not know.                     |
| `invalid-syntax`              | A file that does not parse. It is skipped, the rest is still checked.     |

The codes are named as [ty](https://docs.astral.sh/ty/) names its rules, and the last four are ty's own
codes for the same findings. A finding points to the name of the definition, to the ignore comment or
its code, or to the syntax error, with its line and column.

Local variables and imports are not reported, [Ruff](https://docs.astral.sh/ruff/) covers them.

A definition counts as used when something references it:

- A function, class or variable is read in its own module, imported from it (`from module import name`),
  read through it (`module.name`), or listed in its `__all__`.
- A method or attribute is read as an attribute (`value.name`) anywhere in the project. It is matched by
  name, whatever the object is.

A module is named by the packages it is in, as Python imports it: the directories above it that have an
`__init__.py`, and those without one that are below such a package in the project, which are namespace
packages. The first directory above that is not a package is on the import path.

Code in a function runs only when the function is called, so a reference from an unused function does not
count. A function that only calls itself is unused, and so is a function that only an unused function
calls. The rest of a module runs when it is imported, including class bodies, decorators and default
values, so a reference from there always counts.

Dunders, such as `__init__` and `__all__`, are never reported, and neither is `_`, the name of a value
that is not used, such as the `_: KW_ONLY` field of a data class. A variable named like `_rest` is not
reported where Python needs a name to bind, as in `key, _rest = pair()`, see
[`dummy-variable-rgx`](configuration.md#dummy-variable-rgx).

A method or attribute of a class that has the name of a member of a base class outside the project, such
as `emit` in a subclass of `logging.Handler`, counts as used: the code of the base class uses it. Such a
base class is read from the source of its module, which is never imported, and found in the directories
that the project is imported from, and then in the [environment of the project](configuration.md#python).
A module without a Python source, such as a builtin or a compiled module, is not followed.
A class that a module gives out from its `__getattr__`, such as `concurrent.futures.ThreadPoolExecutor`,
is followed through the `from module import name` of the same name in the function, whatever the
condition it is under.

The pre-commit and prek hook runs in its own environment, which does not have the packages of the
project. It finds them in the `.venv` of the project, or in the environment that the `python` option
names.

## Ignoring a finding

An `# unref: ignore[...]` comment ignores the findings of its line that have one of the codes it names,
as a `# ty: ignore[...]` comment does with ty:

```python
class Setup:
    def __init__(self, name: str) -> None:
        self.name = name  # unref: ignore[unused-attribute]
```

The comment goes on the line of the finding, which for a function or class is the line of its name,
usually that of its `def` or `class`. For names that a framework uses, prefer the
[configuration](configuration.md).

Several codes are separated by commas, as in `# unref: ignore[unused-function, unused-class]`. The
comment can share its line with other comments:

```python
class Plugin:
    hook = None  # type: ignore[assignment]  # unref: ignore[unused-attribute]
```

The comment must name its codes. A bare `# unref: ignore` ignores nothing, and is reported as an
`invalid-ignore-comment`, as is a comment that does not parse. A code that unref does not know is
reported as an `ignore-comment-unknown-rule`.

A comment that ignores nothing is reported as an `unused-ignore-comment`, for example once the definition
is referenced, or covered by the configuration. When some of its codes ignore nothing, each of them is
reported instead.

As with ty, a comment can also ignore the findings about the ignore comments of its line, by their
codes, such as `unused-ignore-comment` for the codes of its line that ignore nothing.
