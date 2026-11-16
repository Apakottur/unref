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
//!   `app.route` or `bp.errorhandler`, of a variable at the top level of a module that is set to
//!   `Flask()` or `Blueprint()`, or to an instance of a subclass of them. It calls the methods of a `MethodView` named after HTTP methods, and the
//!   `dispatch_request` method of a `View`.
//! - Sphinx runs its configuration file, `conf.py`, and reads the settings it sets at the top level,
//!   its own and those of its extensions, which are open-ended, such as `myst_enable_extensions`. It
//!   calls the `setup` function of the file as that of an extension, and `sphinx.ext.linkcode` calls
//!   its `linkcode_resolve` function. Its configuration files are those that the `sphinx.configuration`
//!   option names, or else each `conf.py` that sets `project`, `extensions` or `html_theme` at the top
//!   level, as `sphinx-quickstart` writes. No setting starts with an underscore, so such a name is
//!   left to be reported.
//!
//! pytest and unittest find the methods of a test class through its base classes as well, so a base
//! class of a test class, wherever it is, holds tests.

use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use crate::classes::ClassHierarchy;
use crate::definitions::{Definition, Kind};
use crate::models::{Framework, fnmatch};
use crate::modules::resolve;
use crate::references::LocalCalls;

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
/// The variables that pytest reads from a test module or `conftest.py`, and from a test class.
const PYTEST_MODULE_VARIABLES: [&str; 2] = ["pytestmark", "pytest_plugins"];
const PYTEST_CONFTEST_VARIABLES: [&str; 4] = [
    "pytestmark",
    "pytest_plugins",
    "collect_ignore",
    "collect_ignore_glob",
];
const PYTEST_CLASS_VARIABLES: [&str; 1] = ["pytestmark"];

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

/// The classes of FastAPI's applications and routers, as `fastapi` gives them out, and where they are
/// defined.
const FASTAPI_ROUTERS: [&str; 4] = [
    "fastapi.FastAPI",
    "fastapi.applications.FastAPI",
    "fastapi.APIRouter",
    "fastapi.routing.APIRouter",
];
/// The methods of an application or a router that register the function they decorate.
const FASTAPI_DECORATORS: [&str; 13] = [
    "get",
    "put",
    "post",
    "delete",
    "options",
    "head",
    "patch",
    "trace",
    "api_route",
    "websocket",
    "on_event",
    "middleware",
    "exception_handler",
];

/// The classes of Flask's applications and blueprints, as `flask` gives them out, and where they are
/// defined.
const FLASK_APPS: [&str; 4] = [
    "flask.Flask",
    "flask.app.Flask",
    "flask.Blueprint",
    "flask.blueprints.Blueprint",
];
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
/// The class views of Flask, and the methods of them that Flask calls.
const FLASK_VIEWS: [&str; 1] = ["flask.views.View"];
const FLASK_VIEW_METHODS: [&str; 1] = ["dispatch_request"];
const FLASK_METHOD_VIEWS: [&str; 1] = ["flask.views.MethodView"];
const FLASK_METHOD_VIEW_METHODS: [&str; 8] = [
    "get", "post", "head", "options", "delete", "put", "trace", "patch",
];

/// The name of Sphinx's configuration file, which it reads from its configuration directory.
const SPHINX_CONF: &str = "conf.py";
/// The settings that make a `conf.py` that of Sphinx when it sets one at the top level.
const SPHINX_MARKERS: [&str; 3] = ["project", "extensions", "html_theme"];
/// The functions of the configuration file that Sphinx and its extensions call.
const SPHINX_FUNCTIONS: [&str; 2] = ["setup", "linkcode_resolve"];

/// Check whether a set of dotted names has one of the given names.
fn has_any(names: &FxHashSet<String>, wanted: &[&str]) -> bool {
    wanted.iter().any(|name| names.contains(*name))
}

/// The rules of the frameworks of a project.
pub(crate) struct FrameworkRules<'a> {
    pytest: bool,
    unittest: bool,
    pydantic: bool,
    dataclasses: bool,
    sqlalchemy: bool,
    fastapi: bool,
    flask: bool,
    sphinx: bool,
    /// The module name of each file of the project, by its index.
    module_names: &'a [String],
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
    /// The configuration files of Sphinx, by their indices.
    sphinx_configs: FxHashSet<usize>,
}

/// The instances of some classes, such as the applications of a web framework.
#[derive(Default)]
struct Instances {
    /// The variables at the top level of a module, by their dotted names.
    variables: FxHashSet<String>,
    /// The local variables of functions, each by its module, the line of the name of its function,
    /// and its name.
    locals: FxHashSet<(usize, u32, String)>,
}

/// A file of a project, with its definitions and the local variables that its functions assign the
/// result of a call to.
pub(crate) type ProjectFile<'a> = (&'a Path, &'a [Definition], &'a LocalCalls);

impl<'a> FrameworkRules<'a> {
    /// Make the rules of the given frameworks, for the files of a project, each with its definitions.
    /// The configuration files of Sphinx are the given ones, as resolved paths, or else are found by
    /// their contents.
    pub(crate) fn new(
        frameworks: &FxHashSet<Framework>,
        files: &[ProjectFile],
        module_names: &'a [String],
        hierarchy: &'a ClassHierarchy,
        sphinx_configuration: Option<&FxHashSet<PathBuf>>,
    ) -> Self {
        let mut rules = Self {
            pytest: frameworks.contains(&Framework::Pytest),
            unittest: frameworks.contains(&Framework::Unittest),
            pydantic: frameworks.contains(&Framework::Pydantic),
            dataclasses: frameworks.contains(&Framework::Dataclasses),
            sqlalchemy: frameworks.contains(&Framework::Sqlalchemy),
            fastapi: frameworks.contains(&Framework::Fastapi),
            flask: frameworks.contains(&Framework::Flask),
            sphinx: frameworks.contains(&Framework::Sphinx),
            module_names,
            hierarchy,
            pytest_bases: FxHashSet::default(),
            unittest_bases: FxHashSet::default(),
            mapped_classes: FxHashSet::default(),
            fastapi_routers: Instances::default(),
            flask_apps: Instances::default(),
            sphinx_configs: FxHashSet::default(),
        };
        if rules.fastapi {
            rules.fastapi_routers.locals = local_instances(files, hierarchy, &FASTAPI_ROUTERS);
        }

        // The classes of the project, and the base classes that it makes with SQLAlchemy's function or
        // decorator, by their dotted names.
        let mut classes: Vec<(usize, String)> = Vec::new();
        let mut made: FxHashSet<String> = FxHashSet::default();
        for (module, (path, definitions, _)) in files.iter().enumerate() {
            if rules.sphinx
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
                    if rules.pytest && is_pytest_class(path, &name) {
                        rules
                            .pytest_bases
                            .extend(hierarchy.ancestors(module, &name).iter().cloned());
                    }
                    if rules.unittest && rules.is_test_case(module, &name) {
                        rules
                            .unittest_bases
                            .extend(hierarchy.ancestors(module, &name).iter().cloned());
                    }
                    if rules.sqlalchemy
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
                    if rules.sqlalchemy && is_sqlalchemy(&called, &SQLALCHEMY_BASE_FUNCTIONS) {
                        made.insert(rules.dotted_name(module, &name));
                    }
                    if rules.fastapi && has_any(&called, &FASTAPI_ROUTERS) {
                        let router = rules.dotted_name(module, &name);
                        rules.fastapi_routers.variables.insert(router);
                    }
                    if rules.flask && has_any(&called, &FLASK_APPS) {
                        let app = rules.dotted_name(module, &name);
                        rules.flask_apps.variables.insert(app);
                    }
                }
            }
        }

        if rules.sqlalchemy {
            rules.mapped_classes = rules.sqlalchemy_classes(&classes, &made);
        }
        rules
    }

    /// Check whether a framework finds, calls or reads a definition of a module by itself.
    pub(crate) fn uses(&self, definition: &Definition, module: usize) -> bool {
        (self.pytest && self.pytest_uses(definition, module))
            || (self.unittest && self.unittest_uses(definition, module))
            || (self.pydantic && self.pydantic_uses(definition, module))
            || (self.dataclasses && self.dataclasses_uses(definition, module))
            || (self.sqlalchemy && self.sqlalchemy_uses(definition, module))
            || (self.fastapi
                && self.registers(
                    definition,
                    module,
                    &self.fastapi_routers,
                    &FASTAPI_DECORATORS,
                ))
            || (self.flask && self.flask_uses(definition, module))
            || (self.sphinx && self.sphinx_uses(definition, module))
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
            || definition.decorators.iter().any(|decorator| {
                has_any(&self.hierarchy.resolve(module, decorator), &PYTEST_FIXTURES)
            })
        {
            return true;
        }

        let Some(owner) = &definition.owner else {
            if file_name(path) == PYTEST_CONFTEST {
                return kind == Kind::Variable && PYTEST_CONFTEST_VARIABLES.contains(&name);
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
            || (kind == Kind::Attribute && PYTEST_CLASS_VARIABLES.contains(&name)))
            && (is_pytest_class(path, owner)
                || self.pytest_bases.contains(&self.dotted_name(module, owner)))
    }

    /// Check whether unittest finds and calls a definition by itself.
    fn unittest_uses(&self, definition: &Definition, module: usize) -> bool {
        if definition.kind == Kind::Class {
            return self.is_test_case(module, &definition.qualified_name());
        }
        let name = definition.name.as_str();
        let Some(owner) = &definition.owner else {
            return definition.kind == Kind::Function
                && UNITTEST_MODULE_FUNCTIONS.contains(&name)
                && fnmatch(file_name(&definition.path), UNITTEST_FILES);
        };
        definition.kind == Kind::Method
            && (name.starts_with("test") || UNITTEST_METHODS.contains(&name))
            && (self.is_test_case(module, owner)
                || self
                    .unittest_bases
                    .contains(&self.dotted_name(module, owner)))
    }

    /// Check whether Pydantic calls or reads a definition by itself.
    fn pydantic_uses(&self, definition: &Definition, module: usize) -> bool {
        let Some(owner) = &definition.owner else {
            return false;
        };
        if definition.kind == Kind::Method {
            // A decorator registers the method with Pydantic, in a model or a Pydantic dataclass.
            return definition.decorators.iter().any(|decorator| {
                has_any(
                    &self.hierarchy.resolve(module, decorator),
                    &PYDANTIC_DECORATORS,
                )
            });
        }
        if definition.kind == Kind::Class {
            return definition.name == PYDANTIC_V1_CONFIG && self.is_model(module, owner);
        }

        // An attribute.
        let (model, last) = owner.rsplit_once('.').unwrap_or(("", owner));
        if last == PYDANTIC_V1_CONFIG && !model.is_empty() && self.is_model(module, model) {
            return true;
        }
        self.is_model(module, owner)
            && (definition.name == PYDANTIC_CONFIG
                || (
                    // A field: an annotated name that is not a private attribute or a class variable.
                    definition.annotation.as_ref().is_some_and(|annotation| {
                        !definition.name.starts_with('_')
                            && !has_any(
                                &self.hierarchy.resolve(module, annotation),
                                &CLASS_VARIABLES,
                            )
                    })
                ))
    }

    /// Check whether a class of a module is a subclass of `pydantic.BaseModel`.
    fn is_model(&self, module: usize, name: &str) -> bool {
        has_any(&self.hierarchy.ancestors(module, name), &PYDANTIC_MODELS)
    }

    /// Check whether a dataclass, a named tuple or a class of attrs reads a definition by itself: one
    /// of its fields.
    fn dataclasses_uses(&self, definition: &Definition, module: usize) -> bool {
        let Some(owner) = &definition.owner else {
            return false;
        };
        if definition.kind != Kind::Attribute {
            return false;
        }
        let decorators = self.hierarchy.decorators(module, owner);
        if !has_any(&decorators, &DATACLASS_DECORATORS)
            && !has_any(&self.hierarchy.ancestors(module, owner), &NAMED_TUPLES)
        {
            return false;
        }
        match &definition.annotation {
            Some(annotation) => !has_any(
                &self.hierarchy.resolve(module, annotation),
                &CLASS_VARIABLES,
            ),
            // A field of attrs without an annotation.
            None => definition
                .call
                .as_ref()
                .is_some_and(|call| has_any(&self.hierarchy.resolve(module, call), &ATTRS_FIELDS)),
        }
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
        let Some(owner) = &definition.owner else {
            return false;
        };
        if definition.kind != Kind::Attribute {
            return false;
        }
        self.mapped_classes
            .contains(&self.dotted_name(module, owner))
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
            && let Some(owner) = &definition.owner
        {
            let name = definition.name.as_str();
            let ancestors = self.hierarchy.ancestors(module, owner);
            if (FLASK_METHOD_VIEW_METHODS.contains(&name)
                && has_any(&ancestors, &FLASK_METHOD_VIEWS))
                || (FLASK_VIEW_METHODS.contains(&name)
                    && (has_any(&ancestors, &FLASK_VIEWS)
                        || has_any(&ancestors, &FLASK_METHOD_VIEWS)))
            {
                return true;
            }
        }
        self.registers(definition, module, &self.flask_apps, &FLASK_DECORATORS)
    }

    /// Check whether a definition is a function that a decorator of one of the given instances, such
    /// as the applications of a web framework, registers, with one of the given methods.
    fn registers(
        &self,
        definition: &Definition,
        module: usize,
        instances: &Instances,
        methods: &[&str],
    ) -> bool {
        if !definition.kind.is_function() {
            return false;
        }
        for decorator in &definition.decorators {
            for name in self.hierarchy.resolve(module, decorator).iter() {
                let (instance, method) = name.rsplit_once('.').unwrap_or(("", name));
                // An instance of the module itself is left as it is written, and is named by its
                // module, or is a local variable of the function that the definition is in.
                if methods.contains(&method)
                    && (instances.variables.contains(instance)
                        || (name == decorator
                            && (instances
                                .variables
                                .contains(&self.dotted_name(module, instance))
                                || definition.function.is_some_and(|function| {
                                    instances.locals.contains(&(
                                        module,
                                        function,
                                        instance.to_string(),
                                    ))
                                }))))
                {
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
        has_any(
            &self.hierarchy.ancestors(module, name),
            &UNITTEST_TEST_CASES,
        )
    }

    /// Get the dotted name of a class or variable of a module, as it is among the base classes of a
    /// class.
    fn dotted_name(&self, module: usize, name: &str) -> String {
        format!("{}.{name}", self.module_names[module])
    }
}

/// Find the local variables of the functions of a project that are set to an instance of one of the
/// given classes, or of a subclass of one of them, given by their dotted names.
fn local_instances(
    files: &[ProjectFile],
    hierarchy: &ClassHierarchy,
    classes: &[&str],
) -> FxHashSet<(usize, u32, String)> {
    let mut instances = FxHashSet::default();
    for (module, (_, _, calls)) in files.iter().enumerate() {
        for ((function, name), called) in *calls {
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

/// The name of a file, without its directory.
fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
}

/// Check whether a definition is at the top level of its module, in no class or function.
fn is_top_level(definition: &Definition) -> bool {
    definition.owner.is_none() && definition.function.is_none()
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
