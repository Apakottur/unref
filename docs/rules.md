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

Unused imports and local variables are left to [Ruff](https://docs.astral.sh/ruff/).

A definition counts as used when something references it:

- A function, class or variable: read in its module, imported from it, read as `module.name`, or listed in
  its `__all__`.
- A method or attribute: read as `value.name` anywhere in the project, matched by name.
- A field of a `TypedDict`: read as a key, `value["name"]`, `value.get("name")` or `"name" in value`,
  anywhere in the project, matched by name.
- A member of an enum: read as `Color.RED`, matched by name. Every member, when the enum is read other
  than through a member, such as `Color(value)`, `list(Color)` or as the type of a class attribute. A
  type hint of a parameter or a return reads no member.
- A method or attribute named like a member of a base class from outside the project, such as `emit` in
  a subclass of `logging.Handler`: the base class uses it. The base class is read from the
  [environment of the project](configuration.md#python).

A reference from inside an unused function does not count, so a function that only an unused function
calls is unused too. Dunders and `_` are never reported.

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
