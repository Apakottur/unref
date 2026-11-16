# Recipes

Starting points for frameworks that call code of the project by themselves, to copy into `pyproject.toml`
and adapt.

## Pydantic

Built in: the [`pydantic` framework](configuration.md#pydantic) needs no configuration.

## FastAPI

Built in: the [`fastapi` framework](configuration.md#fastapi) needs no configuration.

## Flask

Built in: the [`flask` framework](configuration.md#flask) needs no configuration.

## pytest

The [`pytest` framework](configuration.md#pytest) follows pytest's default naming. A project that sets
`python_files`, `python_classes` or `python_functions` adds its own names:

```toml
[[tool.unref.overrides]]
paths = ["tests"]
used-names = ["check_*", "Describe*"]
```

## unittest

Built in: the [`unittest` framework](configuration.md#unittest) needs no configuration.
