# FAQ

## What counts as unused?

A definition is unused when no entry point of the project reaches it. The entry points are:

- The code at the top level of every checked file, including tests and scripts.
- The [entry points](configuration.md#entry-points) in `pyproject.toml`.
- What the [frameworks](configuration.md#frameworks) call by themselves, such as pytest's tests.
- The names in [`used-names`](configuration.md#used-names) and the decorators in
  [`used-decorators`](configuration.md#used-decorators).
- The names in the `__all__` of a module.

What an entry point uses is used, and so is what that uses, and so on. Anything else is unused, even if
other unused code references it.

## How do I check a library?

A library is used by code outside the project, so its public API is unused unless the project uses it,
such as in its tests. List the public API in `__all__`, or in [`used-names`](configuration.md#used-names):

```python
__all__ = ["parse", "Parser"]
```
