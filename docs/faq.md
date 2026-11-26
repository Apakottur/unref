# FAQ

## What counts as dead code?

A definition is unused when no entry point of the project reaches it. The entry points are:

- The code at the top level of every checked file, including tests and scripts.
- The [entry points](configuration.md#entry-points) in `pyproject.toml`.
- What the [frameworks](frameworks.md) call by themselves, such as pytest's tests.
- The names in [`used-names`](configuration.md#used-names) and the decorators in
  [`used-decorators`](configuration.md#used-decorators).
- The names in the `__all__` of a module.

What an entry point uses is used, and so is what that uses, and so on. Anything else is unused, even if
other unused code references it.

## How do I adopt unref in an existing project?

A large project can have many findings at first, some of them wrong. Some guidelines:

1. Run `uvx unref` and delete what is really unused.
2. The remaining findings are likely used by:
    - A [framework](frameworks.md) that unref does not cover, or that is turned off.
    - An [entry point](configuration.md#entry-points) of the project that is not configured.
    - Code that finds them by name, such as plugins or handlers: list them in
      [`used-names`](configuration.md#used-names).
3. [Ignore](rules.md#ignoring-a-finding) what is left in the code, and report what unref gets wrong or
   misses in an [issue](https://github.com/Apakottur/unref/issues/new?template=bug_report.yml).
4. Once it is clean, run unref as a [pre-commit hook or in GitHub Actions](usage.md).

## How do I check a library?

A library is used by code outside the project, so its public API is unused unless the project uses it,
such as in its tests. List the public API in `__all__`, or in [`used-names`](configuration.md#used-names):

```python
__all__ = ["parse", "Parser"]
```

## Why is a variable that I increment reported as unused?

Writing to a name is not a use of it, and an augmented assignment such as `retries += 1` writes the
name. It reads the old value only to compute the new one, so a value that is set and updated but never
read is still unused:

```python
retries = 0  # unused-variable
retries += 1
```

The same goes for attributes (`self.count += 1`) and keys (`counts["total"] += 1`). If the update
matters by itself, such as `+=` extending a list that other code also holds, ignore the finding on the
line that first assigns the value:

```python
items = []  # unref: ignore[unused-variable]
items += [1]
```
