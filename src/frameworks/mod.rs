//! The definitions that frameworks find, call or read by themselves, which count as used.
//!
//! The rules follow what the frameworks do with their default settings. Each framework has a module of
//! its own, which describes them.
//!
//! pytest and unittest find the methods of a test class through its base classes as well, so a base
//! class of a test class, wherever it is, holds tests.

mod celery;
mod dataclasses;
mod django;
mod fastapi;
mod flask;
mod pydantic;
mod pytest;
mod sphinx;
mod sqlalchemy;
mod unittest;

use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use crate::classes::ClassHierarchy;
use crate::definitions::{Definition, Kind};
use crate::models::Framework;
use crate::modules::{ModuleName, resolve};
use crate::references::{Index, References};

use celery::{CELERY_APPS, CELERY_CURRENT_APP};
use django::{DjangoModules, django_modules};
use fastapi::FASTAPI_ROUTERS;
use flask::{FLASK_APPS, FLASK_CURRENT_APP, FLASK_GROUPS};
use pytest::is_pytest_class;
use sphinx::is_sphinx_config;
use sqlalchemy::{SQLALCHEMY_BASE_DECORATORS, SQLALCHEMY_BASE_FUNCTIONS, is_sqlalchemy};

/// The HTTP methods, which name methods of FastAPI's applications and routers, and of Flask's
/// `MethodView`.
const HTTP_METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

const CLASS_VARIABLES: [&str; 2] = ["typing.ClassVar", "typing_extensions.ClassVar"];

/// Check whether a set of dotted names has one of the given names.
fn has_any(names: &FxHashSet<String>, wanted: &[&str]) -> bool {
    wanted.iter().any(|name| names.contains(*name))
}

/// The rules of the frameworks of a project.
pub(crate) struct FrameworkRules<'a> {
    /// The frameworks whose rules apply.
    frameworks: &'a FxHashSet<Framework>,
    /// The files of the project, by their indices.
    files: &'a [ProjectFile<'a>],
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

/// The instances of some classes, such as the applications of a web framework.
#[derive(Clone, Default)]
struct Instances {
    /// The variables at the top level of a module, by their dotted names.
    variables: FxHashSet<String>,
    /// The local variables of functions, each by its module, its function, and its name.
    locals: FxHashSet<(usize, Index, String)>,
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
        files: &'a [ProjectFile<'a>],
        module_names: &'a [ModuleName],
        hierarchy: &'a ClassHierarchy,
        sphinx_configuration: Option<&FxHashSet<PathBuf>>,
        django_settings: &FxHashSet<PathBuf>,
    ) -> Self {
        let mut rules = Self {
            frameworks,
            files,
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
            let parameters = rules.flask_fixture_parameters(files);
            rules.flask_apps.locals.extend(parameters);
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
                let name = &definition.qualified_name;
                if definition.kind == Kind::Class {
                    let ancestors = || hierarchy.ancestors(module, name);
                    if rules.enabled(Framework::Pytest) && is_pytest_class(path, name) {
                        rules.pytest_bases.extend(ancestors().iter().cloned());
                    }
                    if rules.enabled(Framework::Unittest) && rules.is_test_case(module, name) {
                        rules.unittest_bases.extend(ancestors().iter().cloned());
                    }
                    if rules.enabled(Framework::Sqlalchemy)
                        && is_sqlalchemy(
                            &hierarchy.decorators(module, name),
                            &SQLALCHEMY_BASE_DECORATORS,
                        )
                    {
                        made.insert(rules.dotted_name(module, name));
                    }
                    classes.push((module, name.clone()));
                } else if definition.kind == Kind::Variable
                    && let Some(call) = &definition.call
                {
                    let called = hierarchy.resolve(module, call);
                    if rules.enabled(Framework::Sqlalchemy)
                        && is_sqlalchemy(&called, &SQLALCHEMY_BASE_FUNCTIONS)
                    {
                        made.insert(rules.dotted_name(module, name));
                    }
                    if rules.enabled(Framework::Fastapi) && has_any(&called, &FASTAPI_ROUTERS) {
                        let router = rules.dotted_name(module, name);
                        rules.fastapi_routers.variables.insert(router);
                    }
                    if rules.enabled(Framework::Flask) {
                        let dotted = rules.dotted_name(module, name);
                        if has_any(&called, &FLASK_APPS) || rules.is_flask_factory(module, call) {
                            rules.flask_apps.variables.insert(dotted);
                        } else if has_any(&called, &FLASK_GROUPS) {
                            rules.flask_groups.variables.insert(dotted);
                        }
                    }
                    if rules.enabled(Framework::Celery) && has_any(&called, &CELERY_APPS) {
                        let app = rules.dotted_name(module, name);
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
            || (self.enabled(Framework::Fastapi) && self.fastapi_uses(definition, module))
            || (self.enabled(Framework::Flask) && self.flask_uses(definition, module))
            || (self.enabled(Framework::Sphinx) && self.sphinx_uses(definition, module))
            || (self.enabled(Framework::Django) && self.django_uses(definition, module))
            || (self.enabled(Framework::Celery) && self.celery_uses(definition, module))
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
        // Check the instance that a decorator is a method of, such as `app` for `app.route`. A method
        // can be that of an attribute of the instance, such as `cli.command`.
        let any_instance = |name: &str, check: &dyn Fn(&str) -> bool| {
            methods
                .iter()
                .flat_map(|methods| methods.iter())
                .filter_map(|method| name.strip_suffix(method)?.strip_suffix('.'))
                .any(check)
        };
        for decorator in &definition.decorators {
            // A local variable or a parameter of the function that the definition is in hides a name
            // of the module, so it is looked for as it is written.
            if let Some(function) = definition.owner
                && any_instance(decorator, &|instance| {
                    instances
                        .locals
                        .contains(&(module, function, instance.to_string()))
                })
            {
                return true;
            }
            for name in self.hierarchy.resolve(module, decorator).iter() {
                // An instance of the module itself is left as it is written, and is named by its
                // module.
                let registered = any_instance(name, &|instance| {
                    instances.variables.contains(instance)
                        || (name == decorator
                            && instances
                                .variables
                                .contains(&self.dotted_name(module, instance)))
                });
                if registered {
                    return true;
                }
            }
        }
        false
    }

    /// Check whether the rules of a framework apply.
    fn enabled(&self, framework: Framework) -> bool {
        self.frameworks.contains(&framework)
    }

    /// Get the name in its module of the class that a definition of a module is in.
    fn class_name(&self, definition: &Definition, module: usize) -> Option<&'a str> {
        let class = definition.class?;
        Some(&self.files[module].1[class.get()].qualified_name)
    }

    /// Get the file of a module.
    fn path(&self, module: usize) -> &'a Path {
        self.files[module].0
    }

    /// Check whether a class of a module inherits from one of the given classes, by their dotted
    /// names.
    fn inherits(&self, module: usize, class: &str, bases: &[&str]) -> bool {
        self.hierarchy.inherits(module, class, bases)
    }

    /// Check whether an expression of a module resolves to one of the given dotted names.
    fn resolves_to(&self, module: usize, expression: &str, names: &[&str]) -> bool {
        self.hierarchy.resolves_to(module, expression, names)
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

    /// Check whether an attribute of a module is annotated, and not as a class variable.
    fn is_field(&self, definition: &Definition, module: usize) -> bool {
        definition
            .annotation
            .as_ref()
            .is_some_and(|annotation| !self.resolves_to(module, annotation, &CLASS_VARIABLES))
    }
}

/// Find the parameters of the functions of a project that are annotated with one of the given classes,
/// or with a subclass of one of them, given by their dotted names.
fn annotated_parameters(
    files: &[ProjectFile],
    hierarchy: &ClassHierarchy,
    classes: &[&str],
) -> FxHashSet<(usize, Index, String)> {
    let mut parameters = FxHashSet::default();
    for (module, (_, _, references)) in files.iter().enumerate() {
        for ((function, name), annotation) in &references.annotated_parameters {
            if hierarchy.resolves_to(module, annotation, classes) {
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
) -> FxHashSet<(usize, Index, String)> {
    let mut instances = FxHashSet::default();
    for (module, (_, _, references)) in files.iter().enumerate() {
        for ((function, name), called) in &references.local_calls {
            if called
                .iter()
                .any(|call| hierarchy.resolves_to(module, call, classes))
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
    definition.class.is_none() && definition.owner.is_none()
}
