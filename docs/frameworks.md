# Frameworks

What a framework finds and calls by itself counts as used, such as pytest's tests or Django's settings.
A framework is turned off in the [configuration](configuration.md#frameworks):

```toml
[tool.unref.frameworks]
unittest = false
```

## `pytest`

Enabled by default: yes.

- In `test_*.py` and `*_test.py` files: `test*` functions, `Test*` classes and their `test*` methods,
  setup and teardown functions.
- Anywhere: `pytest_*` hooks and fixtures.

A project that sets `python_files`, `python_classes` or `python_functions` adds its own names with an
[override](configuration.md#overrides):

```toml
[[tool.unref.overrides]]
paths = ["tests"]
used-names = ["check_*", "Describe*"]
```

## `unittest`

Enabled by default: yes.

- Subclasses of `unittest.TestCase`, with their `test*`, `setUp` and `tearDown` methods.

## `pydantic`

Enabled by default: yes.

- In subclasses of `pydantic.BaseModel`: fields and `model_config`.
- Anywhere: methods with Pydantic's validator and serializer decorators and `computed_field`, also
  when applied by hand, as in `_check = field_validator("name")(check)`.

## `dataclasses`

Enabled by default: yes.

- Fields of classes with `dataclasses.dataclass`, Pydantic's `dataclass` or an attrs decorator such as
  `attrs.define`.
- Fields of `NamedTuple` subclasses.

## `sqlalchemy`

Enabled by default: yes.

- In declarative models and their mixins: attributes annotated `Mapped`, or set to `mapped_column`,
  `Column`, `relationship` and the like.
- Anywhere: functions with `event.listens_for`, `validates`, `reconstructor` or `declared_attr`.

## `fastapi`

Enabled by default: yes.

- Functions and methods with a route or event decorator, such as `app.get` or `router.websocket`, of a
  `FastAPI()` or `APIRouter()`, or an instance of a subclass, set at module level or in the same function.

## `flask`

Enabled by default: yes.

- Functions and methods with a route, error or request decorator, such as `app.route` or
  `bp.errorhandler`, of `Flask()` or `Blueprint()`, or an instance of a subclass, set at module level or
  in the same function, of a parameter annotated as one, of a parameter of a test or fixture named as a
  pytest fixture that returns or yields one, of a module-level variable set to what a factory makes, or
  of `current_app`.
- Commands of `cli.command` and `cli.group` of these, and of `command` and `group` of an `AppGroup()`
  or of such a group.
- Module-level `create_app` and `make_app` functions that make a `Flask()`, and module-level `app` and
  `application` variables set to `Flask()` or to what such a factory makes.
- HTTP method handlers of `MethodView` subclasses, `dispatch_request` of `View` subclasses, and their
  class attributes `methods`, `provide_automatic_options`, `decorators` and `init_every_request`.
- Functions decorated with `after_this_request`, and receivers that `connect` and `connect_via` of
  Flask's signals, such as `request_started`, connect. Not `copy_current_request_context` or
  `stream_with_context`, which only wrap a function.

## `flask-restful`

Enabled by default: yes. Covers Flask-RESTful and Flask-RESTX.

- HTTP method handlers of `Resource` subclasses, and their `method_decorators` and the class
  attributes that `flask` reads of a view. With the package installed in the
  [environment](configuration.md#python), the `flask` rules find them as a `MethodView` as well.
- Classes with the `resource` decorator of an `Api` of Flask-RESTful, or the `route` decorator of an
  `Api` or a `Namespace` of Flask-RESTX.
- Functions with the `representation` decorator of an `Api`, or the `errorhandler` decorator of an
  `Api` or a `Namespace` of Flask-RESTX, or its `documentation` decorator of an `Api`.
- The instances are set at module level or in the same function, or a parameter annotated as one, and
  a `Namespace` can be a module-level variable set to what `api.namespace()` makes.

## `sphinx`

Enabled by default: yes.

- In Sphinx's configuration files: module-level variables not starting with `_`, and the `setup` and
  `linkcode_resolve` functions.

The configuration files are each `conf.py` that sets `project`, `extensions` or `html_theme`, or those that
`configuration` names. A table turns the rules on:

```toml
[tool.unref.frameworks.sphinx]
configuration = ["docs/conf.py"]
```

## `django`

Enabled by default: yes.

- In settings modules: module-level uppercase names, such as `DEBUG`.
- In the module that `ROOT_URLCONF` names: `urlpatterns` and `handler400`, `handler403`, `handler404`,
  `handler500`.
- In modules that `include("...")` names: `urlpatterns` and `app_name`.
- Module-level variables set to `get_wsgi_application()` or `get_asgi_application()`.
- Subclasses of `AppConfig`, with their `ready` method and attributes such as `name`.
- Subclasses of `Migration` named `Migration`, with their attributes such as `operations` and
  `dependencies`.

The settings modules are those that `os.environ.setdefault("DJANGO_SETTINGS_MODULE", ...)` names, those
that `settings` names, and the modules they star import. A table turns the rules on:

```toml
[tool.unref.frameworks.django]
settings = ["config/settings/production.py"]
```

## `celery`

Enabled by default: yes.

- Tasks of `shared_task`, and of the `task` decorator of `Celery()`, or an instance of a subclass, set
  at module level or in the same function, of a parameter annotated as one, or of `current_app`.
- In subclasses of `Task`: `run` and handlers such as `on_failure`.
