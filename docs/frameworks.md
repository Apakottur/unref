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
- HTTP method handlers of `MethodView` subclasses, and `dispatch_request` of `View` subclasses.

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
- In models: fields, such as `models.CharField(...)` or `ForeignKey(...)`, the first manager, the `Meta`
  class with its options, and `get_absolute_url`. In managers: `get_queryset`.
- In forms: fields, `clean_<field>` methods, the `Media` class, and the `Meta` class of a `ModelForm`.
  The `Media` class of widgets and of the admin's classes, such as `ModelAdmin`.
- In class-based views, subclasses of `View`: methods named by HTTP methods, such as `get` and `post`.
- Classes named `Command` in modules of `management/commands` whose name does not start with `_`, and in
  subclasses of `BaseCommand`: `handle`, `add_arguments` and attributes such as `help`.
- Template tags and filters of `register.filter`, `register.simple_tag` and the like, of a module-level
  `Library()`, and a module-level `register` variable set to one.
- Functions with `receiver`.
- In filter sets of `django-filter`: filters, such as `django_filters.CharFilter(...)`, and the `Meta`
  class with its options.

The settings modules are those that `os.environ.setdefault("DJANGO_SETTINGS_MODULE", ...)` names, those
that `settings` names, and the modules they star import. A table turns the rules on:

```toml
[tool.unref.frameworks.django]
settings = ["config/settings/production.py"]
```

## `rest-framework`

Enabled by default: yes.

- In serializers: fields, such as `serializers.CharField(...)` or a nested serializer, `validate_<field>`
  methods, the `get_<field>` method of a `SerializerMethodField`, and the `Meta` class with its options.
- In views, subclasses of `APIView` and the generic views and viewsets: methods named by HTTP methods,
  such as `get`, and the attributes that configure them, such as `serializer_class`, `permission_classes`
  and `filterset_class`.
- In viewsets: the actions `list`, `create`, `retrieve`, `update`, `partial_update` and `destroy`.
- Anywhere: methods with `action`.

## `celery`

Enabled by default: yes.

- Tasks of `shared_task`, and of the `task` decorator of `Celery()`, or an instance of a subclass, set
  at module level or in the same function, of a parameter annotated as one, or of `current_app`.
- In subclasses of `Task`: `run` and handlers such as `on_failure`.
