//! The definitions that frameworks find, call or read by themselves, which count as used.
//!
//! The rules follow what the frameworks do with their default settings:
//!
//! - pytest collects the files that match its `python_files` option, `test_*.py` and `*_test.py`, and
//!   reads `conftest.py`. In such a file, it runs the functions that match `python_functions`,
//!   `test*`, and the methods that match it in the classes that match `python_classes`, `Test*`. It
//!   calls the setup and teardown functions of a module, class or method, the hooks of its plugins,
//!   `pytest_*`, and the fixtures, wherever they are.
//! - unittest runs the methods named `test*` of the subclasses of `unittest.TestCase`, with their setup
//!   and teardown methods. Its `discover` command loads the files that match `test*.py`, and calls
//!   their module setup and teardown, and their `load_tests` function.
//! - Pydantic reads the fields of the subclasses of `pydantic.BaseModel`, the names they annotate other
//!   than class variables and private attributes, and their `model_config`, or their `Config` class in
//!   Pydantic 1. It calls the methods that its validator and serializer decorators register, and reads
//!   the properties that `computed_field` registers.
//! - The dataclasses of `dataclasses`, Pydantic and attrs, and the subclasses of `typing.NamedTuple`,
//!   read their fields in the methods that they generate, such as `__eq__` and `__repr__`, and in
//!   functions such as `dataclasses.asdict`. A field is a name that the class annotates other than a
//!   class variable, or, in a class of attrs, a name that is set to a call of `attr.ib` or
//!   `attrs.field`. A class is a dataclass when it has one of their decorators itself, so a subclass
//!   of a dataclass without one has no fields of its own.
//! - SQLAlchemy maps the attributes of its declarative models: the subclasses of
//!   `sqlalchemy.orm.DeclarativeBase`, `DeclarativeBaseNoMeta` or `MappedAsDataclass`, of a class that
//!   `declarative_base()` makes or that `as_declarative()` decorates, and the classes, such as mixins,
//!   that they inherit from. An attribute is mapped when its annotation is `Mapped`, or its value is a
//!   call of a construct such as `mapped_column`, `Column` or `relationship`. SQLAlchemy calls the
//!   functions that `event.listens_for`, `validates`, `reconstructor` and `declared_attr` register.
//! - FastAPI calls the functions that the decorators of an application or a router register, such as
//!   `app.get` or `router.websocket`: of a variable that is set to `FastAPI()` or `APIRouter()`, or to
//!   an instance of a subclass of them, at the top level of a module, or in the function that the
//!   decorated function is defined in.
//! - Flask calls the functions that the decorators of an application or a blueprint register, such as
//!   `app.route` or `bp.errorhandler`: of a variable that is set to `Flask()` or `Blueprint()`, or to
//!   an instance of a subclass of them, at the top level of a module, or in the function that the
//!   decorated function is defined in, of a parameter of that function that is annotated as one, of a
//!   variable at the top level that is set to what a factory makes, and of `current_app`. It calls the methods of a `MethodView` named after HTTP methods, and
//!   the `dispatch_request` method of a `View`. Its CLI runs the commands that `cli.command` and
//!   `cli.group` of an application or a blueprint add, and those that `command` and `group` of a group
//!   add, a group being an instance of `AppGroup` or a function that such a decorator makes one. It
//!   calls a factory, a function at the top level named `create_app` or `make_app` that sets a local
//!   variable to an application, and loads a variable at the top level named `app` or `application`
//!   that is set to an application or to what a factory makes.
//! - Sphinx runs its configuration file, `conf.py`, and reads the settings it sets at the top level,
//!   its own and those of its extensions, which are open-ended, such as `myst_enable_extensions`. It
//!   calls the `setup` function of the file as that of an extension, and `sphinx.ext.linkcode` calls
//!   its `linkcode_resolve` function. Its configuration files are those that the `configuration`
//!   option of the `frameworks.sphinx` table names, or else each `conf.py` that sets `project`,
//!   `extensions` or `html_theme` at the top level, as `sphinx-quickstart` writes. No setting starts
//!   with an underscore, so such a name is left to be reported.
//! - Django reads the settings of its settings modules: the names at the top level that are all
//!   uppercase, and not the others. Its settings modules are those that `DJANGO_SETTINGS_MODULE` is set
//!   to with `os.environ.setdefault`, as `startproject` writes in `manage.py`, `wsgi.py` and `asgi.py`,
//!   those that the `settings` option of the `frameworks.django` table names, and the modules that they
//!   star import, such as a `base.py` of split settings. It reads `urlpatterns` and the error handlers,
//!   such as `handler404`, from the module that the `ROOT_URLCONF` setting names, and `urlpatterns`
//!   and `app_name` from each module that `include` names by a string. The server loads the
//!   application that `get_wsgi_application()` or `get_asgi_application()` makes, by a dotted path.
//!   Django makes the subclasses of `AppConfig` of the installed applications, reads their attributes
//!   such as `name`, and calls their `ready` method. Its migration commands load the class named
//!   `Migration` of a migration module, a subclass of `Migration`, and read its attributes, such as
//!   `operations`.
//! - Celery's workers run the tasks that `shared_task` registers, and those that the `task` decorator of
//!   an application registers: of a variable that is set to `Celery()`, or to an instance of a subclass
//!   of it, at the top level of a module, or in the function that the decorated function is defined in,
//!   of a parameter of that function that is annotated as one, and of `current_app`. A task runs by its
//!   name, such as from the schedule of Celery beat, so it needs no reference. Celery calls the `run`
//!   method and the handlers, such as `on_failure`, of the subclasses of `Task`.
//!
//! pytest and unittest find the methods of a test class through its base classes as well, so a base
//! class of a test class, wherever it is, holds tests.

use std::path::{Path, PathBuf};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::classes::ClassHierarchy;
use crate::definitions::{Definition, Kind};
use crate::models::{Framework, fnmatch};
use crate::modules::{ModuleName, resolve};
use crate::references::References;

const PYTEST_FILES: [&str; 2] = ["test_*.py", "*_test.py"];
const PYTEST_CONFTEST: &str = "conftest.py";
/// `pytest.fixture` as `pytest` gives it out, and where it is defined, and the same for the fixtures of
/// pytest-asyncio.
const PYTEST_FIXTURES: [&str; 4] = [
    "pytest.fixture",
    "_pytest.fixtures.fixture",
    "pytest_asyncio.fixture",
    "pytest_asyncio.plugin.fixture",
];
/// The functions of a test module that pytest calls, besides the tests.
const PYTEST_MODULE_FUNCTIONS: [&str; 6] = [
    "setup_module",
    "teardown_module",
    "setUpModule",
    "tearDownModule",
    "setup_function",
    "teardown_function",
];
/// The methods of a test class that pytest calls, besides the tests.
const PYTEST_CLASS_METHODS: [&str; 4] = [
    "setup_class",
    "teardown_class",
    "setup_method",
    "teardown_method",
];
/// The variables that pytest reads from a test module or `conftest.py`, of which it reads
/// `pytestmark` from a test class too.
const PYTEST_MODULE_VARIABLES: [&str; 2] = ["pytestmark", "pytest_plugins"];
/// The variables that pytest reads from `conftest.py` only.
const PYTEST_CONFTEST_VARIABLES: [&str; 2] = ["collect_ignore", "collect_ignore_glob"];

const UNITTEST_FILES: &str = "test*.py";
/// `unittest.TestCase` as `unittest` gives it out, and where it is defined.
const UNITTEST_TEST_CASES: [&str; 2] = ["unittest.TestCase", "unittest.case.TestCase"];
const UNITTEST_MODULE_FUNCTIONS: [&str; 3] = ["setUpModule", "tearDownModule", "load_tests"];
const UNITTEST_METHODS: [&str; 6] = [
    "setUp",
    "tearDown",
    "setUpClass",
    "tearDownClass",
    "asyncSetUp",
    "asyncTearDown",
];

/// `pydantic.BaseModel` as `pydantic` gives it out, and where it is defined, also in the Pydantic 1 API
/// of `pydantic.v1`. Packages such as `pydantic_settings` import it from where it is defined.
const PYDANTIC_MODELS: [&str; 4] = [
    "pydantic.BaseModel",
    "pydantic.main.BaseModel",
    "pydantic.v1.BaseModel",
    "pydantic.v1.main.BaseModel",
];
/// The decorators that register a method with Pydantic, as `pydantic` gives them out, and where they
/// are defined, also in the Pydantic 1 API of `pydantic.v1`.
const PYDANTIC_DECORATORS: [&str; 18] = [
    "pydantic.field_validator",
    "pydantic.functional_validators.field_validator",
    "pydantic.model_validator",
    "pydantic.functional_validators.model_validator",
    "pydantic.field_serializer",
    "pydantic.functional_serializers.field_serializer",
    "pydantic.model_serializer",
    "pydantic.functional_serializers.model_serializer",
    "pydantic.computed_field",
    "pydantic.fields.computed_field",
    "pydantic.validator",
    "pydantic.deprecated.class_validators.validator",
    "pydantic.root_validator",
    "pydantic.deprecated.class_validators.root_validator",
    "pydantic.v1.validator",
    "pydantic.v1.class_validators.validator",
    "pydantic.v1.root_validator",
    "pydantic.v1.class_validators.root_validator",
];
const PYDANTIC_CONFIG: &str = "model_config";
/// The configuration class of a model in Pydantic 1.
const PYDANTIC_V1_CONFIG: &str = "Config";
const CLASS_VARIABLES: [&str; 2] = ["typing.ClassVar", "typing_extensions.ClassVar"];

/// The decorators that make a dataclass, as `dataclasses`, Pydantic and attrs give them out, and
/// where they are defined. `attrs` gives out the decorators of `attr`.
const DATACLASS_DECORATORS: [&str; 16] = [
    "dataclasses.dataclass",
    "pydantic.dataclasses.dataclass",
    "attr.s",
    "attr.attrs",
    "attr.attributes",
    "attr.dataclass",
    "attr._make.attrs",
    "attr.define",
    "attr._next_gen.define",
    "attr.mutable",
    "attr._next_gen.mutable",
    "attr.frozen",
    "attr._next_gen.frozen",
    "attrs.define",
    "attrs.mutable",
    "attrs.frozen",
];
/// The classes whose subclasses are named tuples, with a field for each name they annotate.
const NAMED_TUPLES: [&str; 2] = ["typing.NamedTuple", "typing_extensions.NamedTuple"];
/// The functions of attrs that make a field without an annotation, such as `x = attr.ib()`.
const ATTRS_FIELDS: [&str; 7] = [
    "attr.ib",
    "attr.attr",
    "attr.attrib",
    "attr._make.attrib",
    "attr.field",
    "attr._next_gen.field",
    "attrs.field",
];

// SQLAlchemy's names are matched anywhere under `sqlalchemy`, by their end from a dot, as it gives most
// of them out from several modules, such as `sqlalchemy.Column` and `sqlalchemy.schema.Column`.
/// The base classes of declarative models.
const SQLALCHEMY_BASES: [&str; 3] = [
    ".DeclarativeBase",
    ".DeclarativeBaseNoMeta",
    ".MappedAsDataclass",
];
/// The function that makes a base class of declarative models, and the decorator that makes one.
const SQLALCHEMY_BASE_FUNCTIONS: [&str; 1] = [".declarative_base"];
const SQLALCHEMY_BASE_DECORATORS: [&str; 1] = [".as_declarative"];
/// The annotations and constructs of mapped attributes.
const SQLALCHEMY_ANNOTATIONS: [&str; 3] = [".Mapped", ".WriteOnlyMapped", ".DynamicMapped"];
const SQLALCHEMY_CONSTRUCTS: [&str; 9] = [
    ".Column",
    ".mapped_column",
    ".relationship",
    ".column_property",
    ".composite",
    ".synonym",
    ".deferred",
    ".query_expression",
    ".dynamic_loader",
];
/// The decorators that register a function with SQLAlchemy.
const SQLALCHEMY_DECORATORS: [&str; 6] = [
    ".listens_for",
    ".validates",
    ".reconstructor",
    ".declared_attr",
    ".declared_attr.directive",
    ".declared_attr.cascading",
];

/// The HTTP methods, which name methods of FastAPI's applications and routers, and of Flask's
/// `MethodView`.
const HTTP_METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// The classes of FastAPI's applications and routers, as `fastapi` gives them out, and where they are
/// defined.
const FASTAPI_ROUTERS: [&str; 4] = [
    "fastapi.FastAPI",
    "fastapi.applications.FastAPI",
    "fastapi.APIRouter",
    "fastapi.routing.APIRouter",
];
/// The methods of an application or a router that register the function they decorate, besides
/// those named after HTTP methods.
const FASTAPI_DECORATORS: [&str; 5] = [
    "api_route",
    "websocket",
    "on_event",
    "middleware",
    "exception_handler",
];

/// The classes of Flask's applications and blueprints, as `flask` gives them out, and where they are
/// defined.
const FLASK_APPS: [&str; 4] = [
    FLASK_APPLICATIONS[0],
    FLASK_APPLICATIONS[1],
    "flask.Blueprint",
    "flask.blueprints.Blueprint",
];
const FLASK_APPLICATIONS: [&str; 2] = ["flask.Flask", "flask.app.Flask"];
/// `current_app`, which stands for the application, as `flask` gives it out, and where it is defined.
const FLASK_CURRENT_APP: [&str; 2] = ["flask.current_app", "flask.globals.current_app"];
/// The factories and the variables of an application that the Flask CLI finds by their names.
const FLASK_FACTORIES: [&str; 2] = ["create_app", "make_app"];
const FLASK_APP_VARIABLES: [&str; 2] = ["app", "application"];
/// The methods of an application or a blueprint that register the function they decorate.
const FLASK_DECORATORS: [&str; 33] = [
    "route",
    "get",
    "post",
    "put",
    "delete",
    "patch",
    "endpoint",
    "errorhandler",
    "before_request",
    "after_request",
    "teardown_request",
    "context_processor",
    "url_value_preprocessor",
    "url_defaults",
    "teardown_appcontext",
    "shell_context_processor",
    "template_filter",
    "template_test",
    "template_global",
    // A blueprint's decorators that register with the application.
    "app_errorhandler",
    "before_app_request",
    "after_app_request",
    "teardown_app_request",
    "app_context_processor",
    "app_url_value_preprocessor",
    "app_url_defaults",
    "app_template_filter",
    "app_template_test",
    "app_template_global",
    "record",
    "record_once",
    // Removed in Flask 2.3, still in older code.
    "before_first_request",
    "before_app_first_request",
];
/// The decorators of an application or a blueprint that add a command or a group of commands to
/// Flask's CLI.
const FLASK_CLI_DECORATORS: [&str; 2] = ["cli.command", "cli.group"];
/// The classes of the groups of commands of Flask's CLI, as `flask.cli` gives them out, with the
/// methods that add a command or a group to a group.
const FLASK_GROUPS: [&str; 2] = ["flask.cli.AppGroup", "flask.cli.FlaskGroup"];
const FLASK_GROUP_DECORATORS: [&str; 2] = ["command", "group"];
/// The class views of Flask, whose `dispatch_request` method it calls. `MethodView` inherits from
/// `View`, and its `dispatch_request` calls its methods named after HTTP methods.
const FLASK_VIEWS: [&str; 2] = ["flask.views.View", FLASK_METHOD_VIEW];
const FLASK_METHOD_VIEW: &str = "flask.views.MethodView";

/// The name of Sphinx's configuration file, which it reads from its configuration directory.
const SPHINX_CONF: &str = "conf.py";
/// The settings that make a `conf.py` that of Sphinx when it sets one at the top level.
const SPHINX_MARKERS: [&str; 3] = ["project", "extensions", "html_theme"];
/// The functions of the configuration file that Sphinx and its extensions call.
const SPHINX_FUNCTIONS: [&str; 2] = ["setup", "linkcode_resolve"];

/// The environment variable that names the settings module of Django, and `os.environ.setdefault`,
/// which `startproject` sets it with.
const DJANGO_SETTINGS_VARIABLE: &str = "DJANGO_SETTINGS_MODULE";
const OS_ENVIRON_SETDEFAULT: [&str; 1] = ["os.environ.setdefault"];
/// The setting that names the module of the root URLs.
const DJANGO_ROOT_URLCONF: &str = "ROOT_URLCONF";
/// `include`, which names a module of URLs, as `django.urls` gives it out, where it is defined, and as
/// `django.conf.urls` gave it out before Django 4.
const DJANGO_INCLUDES: [&str; 3] = [
    "django.urls.include",
    "django.urls.conf.include",
    "django.conf.urls.include",
];
/// The variables that Django reads from a module of URLs, and those that it reads from the module of
/// the root URLs only, or from an included one only.
const DJANGO_URLS_VARIABLES: [&str; 1] = ["urlpatterns"];
const DJANGO_ROOT_URLS_VARIABLES: [&str; 4] =
    ["handler400", "handler403", "handler404", "handler500"];
const DJANGO_INCLUDED_URLS_VARIABLES: [&str; 1] = ["app_name"];
/// The functions that make the application that a server loads.
const DJANGO_APPLICATIONS: [&str; 2] = [
    "django.core.wsgi.get_wsgi_application",
    "django.core.asgi.get_asgi_application",
];
/// `AppConfig`, as `django.apps` gives it out, and where it is defined, with the attributes and
/// methods of its subclasses that Django reads and calls.
const DJANGO_APP_CONFIGS: [&str; 2] = ["django.apps.AppConfig", "django.apps.config.AppConfig"];
const DJANGO_APP_CONFIG_ATTRIBUTES: [&str; 6] = [
    "name",
    "label",
    "verbose_name",
    "path",
    "default",
    "default_auto_field",
];
const DJANGO_APP_CONFIG_METHODS: [&str; 1] = ["ready"];
/// `Migration`, as `django.db.migrations` gives it out, and where it is defined, with the attributes
/// of its subclasses that Django reads.
const DJANGO_MIGRATIONS: [&str; 2] = [
    "django.db.migrations.Migration",
    "django.db.migrations.migration.Migration",
];
/// The name of the class that Django loads from a migration module.
const DJANGO_MIGRATION_CLASS: &str = "Migration";
const DJANGO_MIGRATION_ATTRIBUTES: [&str; 6] = [
    "operations",
    "dependencies",
    "run_before",
    "replaces",
    "initial",
    "atomic",
];

/// The class of Celery's applications, as `celery` gives it out, and where it is defined.
const CELERY_APPS: [&str; 3] = [
    "celery.Celery",
    "celery.app.Celery",
    "celery.app.base.Celery",
];
/// `current_app`, which stands for the application, as `celery` gives it out, and where it is defined.
const CELERY_CURRENT_APP: [&str; 3] = [
    "celery.current_app",
    "celery.app.current_app",
    "celery._state.current_app",
];
/// The decorator of an application that registers a task.
const CELERY_TASK_DECORATORS: [&str; 1] = ["task"];
/// The decorator that registers a task with every application, as `celery` gives it out, and where it
/// is defined.
const CELERY_SHARED_TASKS: [&str; 2] = ["celery.shared_task", "celery.app.shared_task"];
/// The class of Celery's tasks, as `celery` gives it out, and where it is defined.
const CELERY_TASKS: [&str; 3] = ["celery.Task", "celery.app.Task", "celery.app.task.Task"];
/// The methods of a task that Celery calls.
const CELERY_TASK_METHODS: [&str; 6] = [
    "run",
    "before_start",
    "on_success",
    "on_failure",
    "on_retry",
    "after_return",
];

/// Check whether a set of dotted names has one of the given names.
fn has_any(names: &FxHashSet<String>, wanted: &[&str]) -> bool {
    wanted.iter().any(|name| names.contains(*name))
}

/// The rules of the frameworks of a project.
pub(crate) struct FrameworkRules<'a> {
    /// The frameworks whose rules apply.
    frameworks: &'a FxHashSet<Framework>,
    /// The module name of each file of the project, by its index.
    module_names: &'a [ModuleName],
    hierarchy: &'a ClassHierarchy,
    /// The classes that the test classes inherit from, by their dotted names.
    pytest_bases: FxHashSet<String>,
    unittest_bases: FxHashSet<String>,
    /// The declarative models of SQLAlchemy, and the classes that they inherit from, such as mixins,
    /// by their dotted names.
    mapped_classes: FxHashSet<String>,
    /// The applications and routers of FastAPI.
    fastapi_routers: Instances,
    /// The applications and blueprints of Flask.
    flask_apps: Instances,
    /// The groups of commands of Flask's CLI, as made with `AppGroup` or with a decorator.
    flask_groups: Instances,
    /// The factories of Flask's applications that the Flask CLI finds, by their dotted names.
    flask_factories: FxHashSet<String>,
    /// The configuration files of Sphinx, by their indices.
    sphinx_configs: FxHashSet<usize>,
    /// The modules that Django reads by their names.
    django: DjangoModules,
    /// The applications of Celery.
    celery_apps: Instances,
}

/// The modules that Django reads by their names, by their indices.
#[derive(Default)]
struct DjangoModules {
    /// The settings modules.
    settings: FxHashSet<usize>,
    /// The modules of the root URLs, which `ROOT_URLCONF` names.
    root_urls: FxHashSet<usize>,
    /// The modules of URLs that `include` names.
    included_urls: FxHashSet<usize>,
}

/// The instances of some classes, such as the applications of a web framework.
#[derive(Clone, Default)]
struct Instances {
    /// The variables at the top level of a module, by their dotted names.
    variables: FxHashSet<String>,
    /// The local variables of functions, each by its module, the line of the name of its function,
    /// and its name.
    locals: FxHashSet<(usize, u32, String)>,
}

/// A file of a project, with its definitions and what it refers to.
pub(crate) type ProjectFile<'a> = (&'a Path, &'a [Definition], &'a References);

impl<'a> FrameworkRules<'a> {
    /// Make the rules of the given frameworks, for the files of a project, each with its definitions.
    /// The configuration files of Sphinx are the given ones, as resolved paths, or else are found by
    /// their contents. The settings modules of Django are the given ones, as resolved paths, and those
    /// that are found.
    pub(crate) fn new(
        frameworks: &'a FxHashSet<Framework>,
        files: &[ProjectFile],
        module_names: &'a [ModuleName],
        hierarchy: &'a ClassHierarchy,
        sphinx_configuration: Option<&FxHashSet<PathBuf>>,
        django_settings: &FxHashSet<PathBuf>,
    ) -> Self {
        let mut rules = Self {
            frameworks,
            module_names,
            hierarchy,
            pytest_bases: FxHashSet::default(),
            unittest_bases: FxHashSet::default(),
            mapped_classes: FxHashSet::default(),
            fastapi_routers: Instances::default(),
            flask_apps: Instances::default(),
            flask_groups: Instances::default(),
            flask_factories: FxHashSet::default(),
            sphinx_configs: FxHashSet::default(),
            django: DjangoModules::default(),
            celery_apps: Instances::default(),
        };
        if rules.enabled(Framework::Fastapi) {
            rules.fastapi_routers.locals = local_instances(files, hierarchy, &FASTAPI_ROUTERS);
        }
        if rules.enabled(Framework::Django) {
            rules.django = django_modules(files, module_names, hierarchy, django_settings);
        }
        if rules.enabled(Framework::Flask) {
            rules.flask_apps.locals = local_instances(files, hierarchy, &FLASK_APPS);
            rules
                .flask_apps
                .locals
                .extend(annotated_parameters(files, hierarchy, &FLASK_APPS));
            rules
                .flask_apps
                .variables
                .extend(FLASK_CURRENT_APP.map(String::from));
            rules.flask_factories = rules.flask_factories(files);
            rules.flask_groups.locals = local_instances(files, hierarchy, &FLASK_GROUPS);
        }
        if rules.enabled(Framework::Celery) {
            rules.celery_apps.locals = local_instances(files, hierarchy, &CELERY_APPS);
            rules
                .celery_apps
                .locals
                .extend(annotated_parameters(files, hierarchy, &CELERY_APPS));
            rules
                .celery_apps
                .variables
                .extend(CELERY_CURRENT_APP.map(String::from));
        }

        // The classes of the project, and the base classes that it makes with SQLAlchemy's function or
        // decorator, by their dotted names.
        let mut classes: Vec<(usize, String)> = Vec::new();
        let mut made: FxHashSet<String> = FxHashSet::default();
        for (module, (path, definitions, _)) in files.iter().enumerate() {
            if rules.enabled(Framework::Sphinx)
                && match sphinx_configuration {
                    Some(configuration) => configuration.contains(&resolve(path)),
                    None => is_sphinx_config(path, definitions),
                }
            {
                rules.sphinx_configs.insert(module);
            }
            for definition in *definitions {
                let name = definition.qualified_name();
                if definition.kind == Kind::Class {
                    let ancestors = || hierarchy.ancestors(module, &name);
                    if rules.enabled(Framework::Pytest) && is_pytest_class(path, &name) {
                        rules.pytest_bases.extend(ancestors().iter().cloned());
                    }
                    if rules.enabled(Framework::Unittest) && rules.is_test_case(module, &name) {
                        rules.unittest_bases.extend(ancestors().iter().cloned());
                    }
                    if rules.enabled(Framework::Sqlalchemy)
                        && is_sqlalchemy(
                            &hierarchy.decorators(module, &name),
                            &SQLALCHEMY_BASE_DECORATORS,
                        )
                    {
                        made.insert(rules.dotted_name(module, &name));
                    }
                    classes.push((module, name));
                } else if definition.kind == Kind::Variable
                    && let Some(call) = &definition.call
                {
                    let called = hierarchy.resolve(module, call);
                    if rules.enabled(Framework::Sqlalchemy)
                        && is_sqlalchemy(&called, &SQLALCHEMY_BASE_FUNCTIONS)
                    {
                        made.insert(rules.dotted_name(module, &name));
                    }
                    if rules.enabled(Framework::Fastapi) && has_any(&called, &FASTAPI_ROUTERS) {
                        let router = rules.dotted_name(module, &name);
                        rules.fastapi_routers.variables.insert(router);
                    }
                    if rules.enabled(Framework::Flask) {
                        let dotted = rules.dotted_name(module, &name);
                        if has_any(&called, &FLASK_APPS) || rules.is_flask_factory(module, call) {
                            rules.flask_apps.variables.insert(dotted);
                        } else if has_any(&called, &FLASK_GROUPS) {
                            rules.flask_groups.variables.insert(dotted);
                        }
                    }
                    if rules.enabled(Framework::Celery) && has_any(&called, &CELERY_APPS) {
                        let app = rules.dotted_name(module, &name);
                        rules.celery_apps.variables.insert(app);
                    }
                }
            }
        }

        if rules.enabled(Framework::Sqlalchemy) {
            rules.mapped_classes = rules.sqlalchemy_classes(&classes, &made);
        }
        if rules.enabled(Framework::Flask) {
            rules.flask_groups = rules.flask_command_groups(files);
        }
        rules
    }

    /// Check whether a framework finds, calls or reads a definition of a module by itself.
    pub(crate) fn uses(&self, definition: &Definition, module: usize) -> bool {
        (self.enabled(Framework::Pytest) && self.pytest_uses(definition, module))
            || (self.enabled(Framework::Unittest) && self.unittest_uses(definition, module))
            || (self.enabled(Framework::Pydantic) && self.pydantic_uses(definition, module))
            || (self.enabled(Framework::Dataclasses) && self.dataclasses_uses(definition, module))
            || (self.enabled(Framework::Sqlalchemy) && self.sqlalchemy_uses(definition, module))
            || (self.enabled(Framework::Fastapi)
                && self.registers(
                    definition,
                    module,
                    &self.fastapi_routers,
                    &[&HTTP_METHODS, &FASTAPI_DECORATORS],
                ))
            || (self.enabled(Framework::Flask) && self.flask_uses(definition, module))
            || (self.enabled(Framework::Sphinx) && self.sphinx_uses(definition, module))
            || (self.enabled(Framework::Django) && self.django_uses(definition, module))
            || (self.enabled(Framework::Celery) && self.celery_uses(definition, module))
    }

    /// Check whether Celery runs or calls a definition by itself.
    fn celery_uses(&self, definition: &Definition, module: usize) -> bool {
        if definition.kind == Kind::Method
            && let Some(class) = &definition.class
            && CELERY_TASK_METHODS.contains(&definition.name.as_str())
            && self.inherits(module, class, &CELERY_TASKS)
        {
            return true;
        }
        (definition.kind.is_function()
            && self.is_decorated(definition, module, &CELERY_SHARED_TASKS))
            || self.registers(
                definition,
                module,
                &self.celery_apps,
                &[&CELERY_TASK_DECORATORS],
            )
    }

    /// Check whether Django or a server reads or calls a definition by itself.
    fn django_uses(&self, definition: &Definition, module: usize) -> bool {
        let name = definition.name.as_str();
        if let Some(class) = &definition.class {
            if definition.kind == Kind::Class {
                return false;
            }
            let app_config_member = match definition.kind {
                Kind::Attribute => DJANGO_APP_CONFIG_ATTRIBUTES.contains(&name),
                Kind::Method => DJANGO_APP_CONFIG_METHODS.contains(&name),
                _ => false,
            };
            let migration_member =
                definition.kind == Kind::Attribute && DJANGO_MIGRATION_ATTRIBUTES.contains(&name);
            return (app_config_member && self.inherits(module, class, &DJANGO_APP_CONFIGS))
                || (migration_member && self.inherits(module, class, &DJANGO_MIGRATIONS));
        }
        if definition.kind == Kind::Class {
            return definition.owner.is_none()
                && (self.inherits(module, name, &DJANGO_APP_CONFIGS)
                    || (name == DJANGO_MIGRATION_CLASS
                        && self.inherits(module, name, &DJANGO_MIGRATIONS)));
        }
        if definition.kind != Kind::Variable || definition.owner.is_some() {
            return false;
        }
        let django = &self.django;
        (django.settings.contains(&module) && is_uppercase(name))
            || ((django.root_urls.contains(&module) || django.included_urls.contains(&module))
                && DJANGO_URLS_VARIABLES.contains(&name))
            || (django.root_urls.contains(&module) && DJANGO_ROOT_URLS_VARIABLES.contains(&name))
            || (django.included_urls.contains(&module)
                && DJANGO_INCLUDED_URLS_VARIABLES.contains(&name))
            || definition
                .call
                .as_ref()
                .is_some_and(|call| self.resolves_to(module, call, &DJANGO_APPLICATIONS))
    }

    /// Check whether Sphinx reads or calls a definition of its configuration file by itself.
    fn sphinx_uses(&self, definition: &Definition, module: usize) -> bool {
        let name = definition.name.as_str();
        self.sphinx_configs.contains(&module)
            && is_top_level(definition)
            && ((definition.kind == Kind::Variable && !name.starts_with('_'))
                || (definition.kind == Kind::Function && SPHINX_FUNCTIONS.contains(&name)))
    }

    /// Check whether pytest finds and calls a definition by itself.
    fn pytest_uses(&self, definition: &Definition, module: usize) -> bool {
        let (kind, name, path) = (definition.kind, definition.name.as_str(), &*definition.path);
        // The hooks of a plugin, which can be any module or object that pytest registers, and the
        // fixtures.
        if (kind.is_function() && name.starts_with("pytest_"))
            || self.is_decorated(definition, module, &PYTEST_FIXTURES)
        {
            return true;
        }

        let Some(class) = &definition.class else {
            if file_name(path) == PYTEST_CONFTEST {
                return kind == Kind::Variable
                    && (PYTEST_MODULE_VARIABLES.contains(&name)
                        || PYTEST_CONFTEST_VARIABLES.contains(&name));
            }
            return is_pytest_file(path)
                && ((kind == Kind::Function
                    && (name.starts_with("test") || PYTEST_MODULE_FUNCTIONS.contains(&name)))
                    || (kind == Kind::Class && name.starts_with("Test"))
                    || (kind == Kind::Variable && PYTEST_MODULE_VARIABLES.contains(&name)));
        };

        if kind == Kind::Class {
            return is_pytest_class(path, &definition.qualified_name());
        }
        ((kind == Kind::Method
            && (name.starts_with("test") || PYTEST_CLASS_METHODS.contains(&name)))
            || (kind == Kind::Attribute && name == "pytestmark"))
            && (is_pytest_class(path, class)
                || self.pytest_bases.contains(&self.dotted_name(module, class)))
    }

    /// Check whether unittest finds and calls a definition by itself.
    fn unittest_uses(&self, definition: &Definition, module: usize) -> bool {
        if definition.kind == Kind::Class {
            return self.is_test_case(module, &definition.qualified_name());
        }
        let name = definition.name.as_str();
        let Some(class) = &definition.class else {
            return definition.kind == Kind::Function
                && UNITTEST_MODULE_FUNCTIONS.contains(&name)
                && fnmatch(file_name(&definition.path), UNITTEST_FILES);
        };
        definition.kind == Kind::Method
            && (name.starts_with("test") || UNITTEST_METHODS.contains(&name))
            && (self.is_test_case(module, class)
                || self
                    .unittest_bases
                    .contains(&self.dotted_name(module, class)))
    }

    /// Check whether Pydantic calls or reads a definition by itself.
    fn pydantic_uses(&self, definition: &Definition, module: usize) -> bool {
        let Some(class) = &definition.class else {
            return false;
        };
        if definition.kind == Kind::Method {
            // A decorator registers the method with Pydantic, in a model or a Pydantic dataclass.
            return self.is_decorated(definition, module, &PYDANTIC_DECORATORS);
        }
        if definition.kind == Kind::Class {
            return definition.name == PYDANTIC_V1_CONFIG && self.is_model(module, class);
        }

        // An attribute, which a decorator applied by hand can register as it does a method.
        if definition
            .wrapped_by
            .as_ref()
            .is_some_and(|decorator| self.resolves_to(module, decorator, &PYDANTIC_DECORATORS))
        {
            return true;
        }
        let (model, last) = class.rsplit_once('.').unwrap_or(("", class));
        if last == PYDANTIC_V1_CONFIG && !model.is_empty() && self.is_model(module, model) {
            return true;
        }
        self.is_model(module, class)
            && (definition.name == PYDANTIC_CONFIG
                // A field that is not a private attribute.
                || (!definition.name.starts_with('_') && self.is_field(definition, module)))
    }

    /// Check whether a class of a module is a subclass of `pydantic.BaseModel`.
    fn is_model(&self, module: usize, name: &str) -> bool {
        self.inherits(module, name, &PYDANTIC_MODELS)
    }

    /// Check whether a dataclass, a named tuple or a class of attrs reads a definition by itself: one
    /// of its fields.
    fn dataclasses_uses(&self, definition: &Definition, module: usize) -> bool {
        let Some(class) = &definition.class else {
            return false;
        };
        if definition.kind != Kind::Attribute {
            return false;
        }
        let decorators = self.hierarchy.decorators(module, class);
        if !has_any(&decorators, &DATACLASS_DECORATORS)
            && !self.inherits(module, class, &NAMED_TUPLES)
        {
            return false;
        }
        self.is_field(definition, module)
            // A field of attrs without an annotation.
            || (definition.annotation.is_none()
                && definition
                    .call
                    .as_ref()
                    .is_some_and(|call| self.resolves_to(module, call, &ATTRS_FIELDS)))
    }

    /// Check whether an attribute of a module is annotated, and not as a class variable.
    fn is_field(&self, definition: &Definition, module: usize) -> bool {
        definition
            .annotation
            .as_ref()
            .is_some_and(|annotation| !self.resolves_to(module, annotation, &CLASS_VARIABLES))
    }

    /// Check whether SQLAlchemy maps or calls a definition by itself.
    fn sqlalchemy_uses(&self, definition: &Definition, module: usize) -> bool {
        if definition.kind.is_function() {
            return definition.decorators.iter().any(|decorator| {
                is_sqlalchemy(
                    &self.hierarchy.resolve(module, decorator),
                    &SQLALCHEMY_DECORATORS,
                )
            });
        }
        let Some(class) = &definition.class else {
            return false;
        };
        if definition.kind != Kind::Attribute {
            return false;
        }
        self.mapped_classes
            .contains(&self.dotted_name(module, class))
            && (definition.annotation.as_ref().is_some_and(|annotation| {
                is_sqlalchemy(
                    &self.hierarchy.resolve(module, annotation),
                    &SQLALCHEMY_ANNOTATIONS,
                )
            }) || definition.call.as_ref().is_some_and(|call| {
                is_sqlalchemy(
                    &self.hierarchy.resolve(module, call),
                    &SQLALCHEMY_CONSTRUCTS,
                )
            }))
    }

    /// Check whether Flask calls a definition by itself.
    fn flask_uses(&self, definition: &Definition, module: usize) -> bool {
        if definition.kind == Kind::Method
            && let Some(class) = &definition.class
        {
            let name = definition.name.as_str();
            if (HTTP_METHODS.contains(&name) && self.inherits(module, class, &[FLASK_METHOD_VIEW]))
                || (name == "dispatch_request" && self.inherits(module, class, &FLASK_VIEWS))
            {
                return true;
            }
        }
        if is_top_level(definition) {
            let name = definition.name.as_str();
            if definition.kind == Kind::Function && FLASK_FACTORIES.contains(&name) {
                return self
                    .flask_factories
                    .contains(&self.dotted_name(module, name));
            }
            if definition.kind == Kind::Variable && FLASK_APP_VARIABLES.contains(&name) {
                return definition.call.as_ref().is_some_and(|call| {
                    self.resolves_to(module, call, &FLASK_APPLICATIONS)
                        || self.is_flask_factory(module, call)
                });
            }
        }
        self.registers(
            definition,
            module,
            &self.flask_apps,
            &[&FLASK_DECORATORS, &FLASK_CLI_DECORATORS],
        ) || self.registers(
            definition,
            module,
            &self.flask_groups,
            &[&FLASK_GROUP_DECORATORS],
        )
    }

    /// Find the factories of Flask's applications that the Flask CLI finds: the functions at the top
    /// level of a module that are named as one, and set a local variable to an application.
    fn flask_factories(&self, files: &[ProjectFile]) -> FxHashSet<String> {
        let applications: FxHashSet<(usize, u32)> =
            local_instances(files, self.hierarchy, &FLASK_APPLICATIONS)
                .into_iter()
                .map(|(module, function, _)| (module, function))
                .collect();
        let mut factories = FxHashSet::default();
        for (module, (_, definitions, _)) in files.iter().enumerate() {
            for definition in *definitions {
                if definition.kind == Kind::Function
                    && is_top_level(definition)
                    && FLASK_FACTORIES.contains(&definition.name.as_str())
                    && applications.contains(&(module, definition.line))
                {
                    factories.insert(self.dotted_name(module, &definition.name));
                }
            }
        }
        factories
    }

    /// Check whether a call of a module calls a factory of Flask's applications.
    fn is_flask_factory(&self, module: usize, call: &str) -> bool {
        self.flask_factories
            .contains(&self.dotted_name(module, call))
            || self
                .hierarchy
                .resolve(module, call)
                .iter()
                .any(|name| self.flask_factories.contains(name))
    }

    /// Get the groups of commands of Flask's CLI: the instances of `AppGroup`, and the functions that
    /// the `cli.group` decorator of an application or a blueprint, or the `group` decorator of a group,
    /// makes into a group, until no more are found.
    fn flask_command_groups(&self, files: &[ProjectFile]) -> Instances {
        let mut groups = self.flask_groups.clone();
        loop {
            let mut found = Instances::default();
            for (module, (_, definitions, _)) in files.iter().enumerate() {
                for definition in *definitions {
                    if definition.kind != Kind::Function
                        || !(self.registers(
                            definition,
                            module,
                            &self.flask_apps,
                            &[&["cli.group"]],
                        ) || self.registers(definition, module, &groups, &[&["group"]]))
                    {
                        continue;
                    }
                    let name = definition.name.clone();
                    match definition.owner {
                        None => found.variables.insert(self.dotted_name(module, &name)),
                        Some(function) => found.locals.insert((module, function, name)),
                    };
                }
            }
            let count = groups.variables.len() + groups.locals.len();
            groups.variables.extend(found.variables);
            groups.locals.extend(found.locals);
            if groups.variables.len() + groups.locals.len() == count {
                return groups;
            }
        }
    }

    /// Check whether a definition is a function that a decorator of one of the given instances, such
    /// as the applications of a web framework, registers, with a method of one of the given lists.
    fn registers(
        &self,
        definition: &Definition,
        module: usize,
        instances: &Instances,
        methods: &[&[&str]],
    ) -> bool {
        if !definition.kind.is_function() {
            return false;
        }
        for decorator in &definition.decorators {
            for name in self.hierarchy.resolve(module, decorator).iter() {
                // A method can be that of an attribute of the instance, such as `cli.command`.
                let registered = methods
                    .iter()
                    .flat_map(|methods| methods.iter())
                    .any(|method| {
                        let Some(instance) = name
                            .strip_suffix(method)
                            .and_then(|instance| instance.strip_suffix('.'))
                        else {
                            return false;
                        };
                        // An instance of the module itself is left as it is written, and is named
                        // by its module, or is a local variable of the function that the definition
                        // is in.
                        instances.variables.contains(instance)
                            || (name == decorator
                                && (instances
                                    .variables
                                    .contains(&self.dotted_name(module, instance))
                                    || definition.owner.is_some_and(|function| {
                                        instances.locals.contains(&(
                                            module,
                                            function,
                                            instance.to_string(),
                                        ))
                                    })))
                    });
                if registered {
                    return true;
                }
            }
        }
        false
    }

    /// Get the dotted names of the declarative models of SQLAlchemy, and of the classes that they
    /// inherit from, given the classes of the project and the base classes that it makes.
    fn sqlalchemy_classes(
        &self,
        classes: &[(usize, String)],
        made: &FxHashSet<String>,
    ) -> FxHashSet<String> {
        // A variable that a class inherits from in its own module is among its base classes by its name
        // alone.
        let mut models = made.clone();
        for (module, name) in classes {
            let ancestors = self.hierarchy.ancestors(*module, name);
            let local = ancestors
                .iter()
                .filter(|ancestor| !ancestor.contains('.'))
                .map(|ancestor| self.dotted_name(*module, ancestor));
            let inherits_made = ancestors.iter().any(|ancestor| made.contains(ancestor))
                || local.into_iter().any(|ancestor| made.contains(&ancestor));
            if is_sqlalchemy(&ancestors, &SQLALCHEMY_BASES) || inherits_made {
                models.insert(self.dotted_name(*module, name));
            }
        }

        // A model can inherit from a model of another module, whose base class is known only there.
        let mut mapped = models.clone();
        for (module, name) in classes {
            let ancestors = self.hierarchy.ancestors(*module, name);
            let dotted = self.dotted_name(*module, name);
            if models.contains(&dotted)
                || ancestors.iter().any(|ancestor| models.contains(ancestor))
            {
                mapped.insert(dotted);
                mapped.extend(ancestors.iter().cloned());
            }
        }
        mapped
    }

    /// Check whether a class of a module is a subclass of `unittest.TestCase`.
    fn is_test_case(&self, module: usize, name: &str) -> bool {
        self.inherits(module, name, &UNITTEST_TEST_CASES)
    }

    /// Check whether the rules of a framework apply.
    fn enabled(&self, framework: Framework) -> bool {
        self.frameworks.contains(&framework)
    }

    /// Check whether a class of a module inherits from one of the given classes, by their dotted
    /// names.
    fn inherits(&self, module: usize, class: &str, bases: &[&str]) -> bool {
        has_any(&self.hierarchy.ancestors(module, class), bases)
    }

    /// Check whether an expression of a module resolves to one of the given dotted names.
    fn resolves_to(&self, module: usize, expression: &str, names: &[&str]) -> bool {
        has_any(&self.hierarchy.resolve(module, expression), names)
    }

    /// Check whether a definition of a module has a decorator that resolves to one of the given
    /// dotted names.
    fn is_decorated(&self, definition: &Definition, module: usize, names: &[&str]) -> bool {
        definition
            .decorators
            .iter()
            .any(|decorator| self.resolves_to(module, decorator, names))
    }

    /// Get the dotted name of a class or variable of a module, as it is among the base classes of a
    /// class.
    fn dotted_name(&self, module: usize, name: &str) -> String {
        format!("{}.{name}", self.module_names[module].name)
    }
}

/// Find the parameters of the functions of a project that are annotated with one of the given classes,
/// or with a subclass of one of them, given by their dotted names.
fn annotated_parameters(
    files: &[ProjectFile],
    hierarchy: &ClassHierarchy,
    classes: &[&str],
) -> FxHashSet<(usize, u32, String)> {
    let mut parameters = FxHashSet::default();
    for (module, (_, _, references)) in files.iter().enumerate() {
        for ((function, name), annotation) in &references.annotated_parameters {
            if has_any(&hierarchy.resolve(module, annotation), classes) {
                parameters.insert((module, *function, name.clone()));
            }
        }
    }
    parameters
}

/// Find the local variables of the functions of a project that are set to an instance of one of the
/// given classes, or of a subclass of one of them, given by their dotted names.
fn local_instances(
    files: &[ProjectFile],
    hierarchy: &ClassHierarchy,
    classes: &[&str],
) -> FxHashSet<(usize, u32, String)> {
    let mut instances = FxHashSet::default();
    for (module, (_, _, references)) in files.iter().enumerate() {
        for ((function, name), called) in &references.local_calls {
            if called
                .iter()
                .any(|call| has_any(&hierarchy.resolve(module, call), classes))
            {
                instances.insert((module, *function, name.clone()));
            }
        }
    }
    instances
}

/// Find the modules that Django reads by their names, given the settings modules that are given by
/// their resolved paths.
fn django_modules(
    files: &[ProjectFile],
    module_names: &[ModuleName],
    hierarchy: &ClassHierarchy,
    given: &FxHashSet<PathBuf>,
) -> DjangoModules {
    let mut by_name: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
    for (module, name) in module_names.iter().enumerate() {
        by_name.entry(name.name.as_str()).or_default().push(module);
    }
    let modules = |name: &str| by_name.get(name).into_iter().flatten().copied();

    let mut pending: Vec<usize> = Vec::new();
    let mut included_urls = FxHashSet::default();
    for (module, (path, _, references)) in files.iter().enumerate() {
        if !given.is_empty() && given.contains(&resolve(path)) {
            pending.push(module);
        }
        for (called, arguments) in &references.string_calls {
            let resolves_to = |names: &[&str]| has_any(&hierarchy.resolve(module, called), names);
            match arguments.as_slice() {
                [variable, settings, ..]
                    if variable == DJANGO_SETTINGS_VARIABLE
                        && resolves_to(&OS_ENVIRON_SETDEFAULT) =>
                {
                    pending.extend(modules(settings));
                }
                [urls, ..] if resolves_to(&DJANGO_INCLUDES) => included_urls.extend(modules(urls)),
                _ => {}
            }
        }
    }

    // The modules that a settings module star imports give out their settings through it.
    let mut settings = FxHashSet::default();
    while let Some(module) = pending.pop() {
        if !settings.insert(module) {
            continue;
        }
        for import in &files[module].2.imports {
            if import.name == "*"
                && let Some(name) =
                    module_names[module].resolve(import.module.as_deref(), import.level)
            {
                pending.extend(modules(&name));
            }
        }
    }

    let mut root_urls = FxHashSet::default();
    for &module in &settings {
        for definition in files[module].1 {
            if definition.kind == Kind::Variable
                && is_top_level(definition)
                && definition.name == DJANGO_ROOT_URLCONF
                && let Some(urls) = &definition.string
            {
                root_urls.extend(modules(urls));
            }
        }
    }
    DjangoModules {
        settings,
        root_urls,
        included_urls,
    }
}

/// Check whether a name is uppercase, as Python's `str.isupper`: it has a cased letter, and no
/// lowercase one.
fn is_uppercase(name: &str) -> bool {
    name.chars().any(char::is_uppercase) && !name.chars().any(char::is_lowercase)
}

/// The name of a file, without its directory.
fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
}

/// Check whether a definition is at the top level of its module, in no class or function.
fn is_top_level(definition: &Definition) -> bool {
    definition.class.is_none() && definition.owner.is_none()
}

/// Check whether a file is the configuration file of Sphinx: a `conf.py` that sets one of Sphinx's
/// marker settings at the top level.
fn is_sphinx_config(path: &Path, definitions: &[Definition]) -> bool {
    file_name(path) == SPHINX_CONF
        && definitions.iter().any(|definition| {
            definition.kind == Kind::Variable
                && is_top_level(definition)
                && SPHINX_MARKERS.contains(&definition.name.as_str())
        })
}

/// Check whether pytest collects the tests of a file.
fn is_pytest_file(path: &Path) -> bool {
    PYTEST_FILES
        .iter()
        .any(|pattern| fnmatch(file_name(path), pattern))
}

/// Check whether pytest collects a class of a module: a class of a test file whose name starts with
/// `Test`, at the top level or in such a class.
fn is_pytest_class(path: &Path, name: &str) -> bool {
    is_pytest_file(path) && name.split('.').all(|part| part.starts_with("Test"))
}

/// Check whether one of the dotted names of something is a name of SQLAlchemy that ends with one of
/// the given ends, such as `sqlalchemy.orm.mapped_column` for `.mapped_column`.
fn is_sqlalchemy(names: &FxHashSet<String>, ends: &[&str]) -> bool {
    names
        .iter()
        .any(|name| name.starts_with("sqlalchemy.") && ends.iter().any(|end| name.ends_with(end)))
}
