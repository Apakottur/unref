# Recipes

Starting points for frameworks that call code of the project by themselves, to copy into `pyproject.toml`
and adapt.

## Pydantic

Pydantic reads the fields of a model:

```toml
[[tool.unref.overrides]]
base-classes = ["pydantic.BaseModel", "pydantic.main.BaseModel"]
used-names = ["*"]
```

The second name covers subclasses from other packages, such as `pydantic_settings.BaseSettings`, which
import it from `pydantic.main`.

## FastAPI

FastAPI calls the functions that its decorators register:

```toml
[tool.unref]
used-decorators = ["app.get", "app.post", "router.get", "router.post"]
```

A decorator is matched by its name as written: add the methods and the names of the `FastAPI` and
`APIRouter` objects your project uses, such as `users_router.delete` or `app.websocket`. Add the
[Pydantic](#pydantic) recipe for the models.

## pytest

The [`pytest` framework](configuration.md#frameworks) follows pytest's default naming. A project that sets
`python_files`, `python_classes` or `python_functions` adds its own names:

```toml
[[tool.unref.overrides]]
paths = ["tests"]
used-names = ["check_*", "Describe*"]
```
