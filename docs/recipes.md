# Recipes

Configurations for common frameworks, to copy into the `[tool.unref]` table of your `pyproject.toml`. See
[Configuration](configuration.md) for what each option does.

A framework calls some of the code of the project itself, so nothing in the project references it. Without
configuration, `unref` reports that code as unused.

## FastAPI

```toml
[tool.unref]
used-decorators = [
  "app.get", "app.post", "app.put", "app.patch", "app.delete", "app.head", "app.options", "app.trace",
  "app.api_route", "app.websocket", "app.middleware", "app.exception_handler", "app.on_event",
  "router.get", "router.post", "router.put", "router.patch", "router.delete", "router.head",
  "router.options", "router.trace", "router.api_route", "router.websocket", "router.on_event",
]

[[tool.unref.overrides]]
base-classes = ["pydantic.BaseModel", "pydantic.main.BaseModel"]
used-names = ["*"]
```

- **Path operations**: FastAPI calls the functions that `@app.get(...)` and the other decorators
  register. A decorator is matched by its name as written, so the recipe covers a `FastAPI` instance
  named `app` and an `APIRouter` named `router`. Add the names your project uses, such as `api.get` or
  `users_router.post`.
- **Models**: Pydantic reads the fields of a model, for a request body or a response. The override counts
  everything in a model as used, its methods too. It also covers a subclass of a model from another
  package, such as `pydantic_settings.BaseSettings`, when that package is installed in the
  [environment of the project](configuration.md#python). Such a package imports the class as
  `pydantic.main.BaseModel`, hence the second name.

Functions passed to `Depends(...)`, and a `lifespan` passed to `FastAPI(...)`, are referenced by name,
and need no configuration.
