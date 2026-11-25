//! The definitions that frameworks find, call or read by themselves, which count as used.
//!
//! The rules follow what the frameworks do with their default settings. Each framework has a module of
//! its own, which describes them.
//!
//! pytest and unittest find the methods of a test class through its base classes as well, so a base
//! class of a test class, wherever it is, holds tests.
//!
//! Each framework is a type that implements [`Rules`]: it is made from a [`Context`], a read-only view
//! of the project that the frameworks share, learns from the definitions of the project in one pass
//! that serves every framework, and then answers whether it uses a definition. A framework whose rules
//! do not apply is not made, so it does no work. To add a framework, add a module for it here, a
//! variant with its name to `frameworks!` in `models.rs`, and an arm that makes it to `make`.

mod celery;
mod click;
mod dataclasses;
mod django;
mod fastapi;
mod flask;
mod flask_restful;
mod pydantic;
mod pytest;
mod rest_framework;
mod sphinx;
mod sqlalchemy;
mod stdlib;
mod typer;
mod unittest;

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use crate::classes::ClassHierarchy;
use crate::definitions::{Definition, Kind};
use crate::models::Framework;
use crate::modules::ModuleName;
use crate::references::{Hints, Index, References};
use crate::resolver::Names;

/// The HTTP methods, which name methods of FastAPI's applications and routers, of Flask's
/// `MethodView`, and of the resources of Flask-RESTful and Flask-RESTX.
const HTTP_METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

const CLASS_VARIABLES: [&str; 2] = ["typing.ClassVar", "typing_extensions.ClassVar"];

/// Check whether a set of dotted names has one of the given names.
fn has_any(names: &FxHashSet<String>, wanted: &[&str]) -> bool {
    wanted.iter().any(|name| names.contains(*name))
}

/// Check whether a set of dotted names has a name in a package whose last part ends with one of the
/// given ends, such as `rest_framework.fields.CharField` in `rest_framework` for `Field`.
fn has_end(names: &FxHashSet<String>, package: &str, ends: &[&str]) -> bool {
    names.iter().any(|name| {
        name.strip_prefix(package)
            .and_then(|rest| rest.strip_prefix('.'))
            .and_then(|rest| rest.rsplit('.').next())
            .is_some_and(|last| ends.iter().any(|end| last.ends_with(end)))
    })
}

/// A file of a project, with its definitions, what it refers to, and its hints for frameworks.
pub(crate) struct ProjectFile<'a> {
    pub path: &'a Path,
    pub definitions: &'a [Definition],
    pub references: &'a References,
    pub hints: &'a Hints,
}

/// The settings of the frameworks that the options give.
pub(crate) struct Settings<'a> {
    /// The configuration files of Sphinx, as resolved paths, or None to find them by their contents.
    pub sphinx_configuration: Option<&'a FxHashSet<PathBuf>>,
    /// The settings modules of Django that are given, as resolved paths, besides those that are found.
    pub django_settings: &'a FxHashSet<PathBuf>,
}

/// The project as the frameworks see it, which they share, with what they ask of it.
pub(crate) struct Context<'a> {
    /// The files of the project, by their indices.
    files: &'a [ProjectFile<'a>],
    /// The module name of each file of the project, by its index.
    module_names: &'a [ModuleName],
    hierarchy: &'a ClassHierarchy,
}

/// The rules of one framework.
///
/// The framework is made with `new`, which can look at the whole project. Then each file of the project
/// is passed to `file`, followed by its classes to `class` and its variables that are assigned the
/// result of a call to `variable_call`, in the order of the files. Then `finish` is called, and from
/// then on `uses`.
trait Rules {
    /// Make the rules of the framework for a project.
    fn new(context: &Context, settings: &Settings) -> Self
    where
        Self: Sized;

    /// Learn from a file of the project, by its index, before its definitions.
    fn file(&mut self, _context: &Context, _module: usize) {}

    /// Learn from a class of a module.
    fn class(&mut self, _context: &Context, _module: usize, _class: &Definition) {}

    /// Learn from a variable of a module that is first assigned the result of a call.
    fn variable_call(
        &mut self,
        _context: &Context,
        _module: usize,
        _variable: &Definition,
        _call: &Call,
    ) {
    }

    /// Learn what needs every definition of the project to have been seen.
    fn finish(&mut self, _context: &Context) {}

    /// Check whether the framework finds, calls or reads a definition of a module by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool;
}

/// Make the rules of a framework.
fn make(framework: Framework, context: &Context, settings: &Settings) -> Box<dyn Rules> {
    match framework {
        Framework::Pytest => Box::new(pytest::Pytest::new(context, settings)),
        Framework::Unittest => Box::new(unittest::Unittest::new(context, settings)),
        Framework::Stdlib => Box::new(stdlib::Stdlib::new(context, settings)),
        Framework::Pydantic => Box::new(pydantic::Pydantic::new(context, settings)),
        Framework::Dataclasses => Box::new(dataclasses::Dataclasses::new(context, settings)),
        Framework::Sqlalchemy => Box::new(sqlalchemy::Sqlalchemy::new(context, settings)),
        Framework::Fastapi => Box::new(fastapi::Fastapi::new(context, settings)),
        Framework::Flask => Box::new(flask::Flask::new(context, settings)),
        Framework::FlaskRestful => Box::new(flask_restful::FlaskRestful::new(context, settings)),
        Framework::Sphinx => Box::new(sphinx::Sphinx::new(context, settings)),
        Framework::Django => Box::new(django::Django::new(context, settings)),
        Framework::Drf => Box::new(rest_framework::RestFramework::new(context, settings)),
        Framework::Celery => Box::new(celery::Celery::new(context, settings)),
        Framework::Click => Box::new(click::Click::new(context, settings)),
        Framework::Typer => Box::new(typer::Typer::new(context, settings)),
    }
}

/// What a variable is first assigned the result of calling: its dotted name as it is written, and the
/// dotted names that it resolves to, which are found once, when a framework first asks for them.
struct Call<'c> {
    written: &'c str,
    resolved: OnceCell<Names>,
}

impl Call<'_> {
    /// Get the dotted names that the call resolves to in a module.
    fn resolved(&self, context: &Context, module: usize) -> &FxHashSet<String> {
        self.resolved
            .get_or_init(|| context.hierarchy.resolve(module, self.written))
    }
}

/// The rules of the frameworks of a project.
pub(crate) struct FrameworkRules<'a> {
    context: Context<'a>,
    /// The rules of each framework that applies, in the order of `Framework::ALL`.
    rules: Vec<Box<dyn Rules>>,
}

impl<'a> FrameworkRules<'a> {
    /// Make the rules of the given frameworks, for the files of a project, each with its definitions.
    pub(crate) fn new(
        frameworks: &FxHashSet<Framework>,
        files: &'a [ProjectFile<'a>],
        module_names: &'a [ModuleName],
        hierarchy: &'a ClassHierarchy,
        settings: &Settings,
    ) -> Self {
        let context = Context {
            files,
            module_names,
            hierarchy,
        };
        let mut rules: Vec<Box<dyn Rules>> = Framework::ALL
            .iter()
            .copied()
            .filter(|framework| frameworks.contains(framework))
            .map(|framework| make(framework, &context, settings))
            .collect();
        if !rules.is_empty() {
            // One pass over the definitions serves every framework.
            for (module, file) in files.iter().enumerate() {
                for framework in &mut rules {
                    framework.file(&context, module);
                }
                for definition in file.definitions {
                    if definition.kind == Kind::Class {
                        for framework in &mut rules {
                            framework.class(&context, module, definition);
                        }
                    } else if definition.kind == Kind::Variable
                        && let Some(written) = &definition.call
                    {
                        let call = Call {
                            written,
                            resolved: OnceCell::new(),
                        };
                        for framework in &mut rules {
                            framework.variable_call(&context, module, definition, &call);
                        }
                    }
                }
            }
            for framework in &mut rules {
                framework.finish(&context);
            }
        }
        Self { context, rules }
    }

    /// Check whether a framework finds, calls or reads a definition of a module by itself.
    pub(crate) fn uses(&self, definition: &Definition, module: usize) -> bool {
        self.rules
            .iter()
            .any(|framework| framework.uses(&self.context, definition, module))
    }
}

/// The instances of some classes, such as the applications of a web framework.
#[derive(Clone, Default)]
struct Instances {
    /// The variables at the top level of a module, by their dotted names.
    variables: FxHashSet<String>,
    /// The local variables of functions, each by its module, its function, and its name.
    locals: FxHashSet<(usize, Index, String)>,
}

// The small helpers are inlined, as the rules of each framework call them for each definition of the
// project.
impl<'a> Context<'a> {
    /// Check whether a definition is a function that a decorator of one of the given instances, such
    /// as the applications of a web framework, registers, with a method of one of the given lists.
    fn registers(
        &self,
        definition: &Definition,
        module: usize,
        instances: &Instances,
        methods: &[&[&str]],
    ) -> bool {
        definition.kind.is_function() && self.decorated_by(definition, module, instances, methods)
    }

    /// Check whether a definition, a function or a class, has a decorator that is a method of one of
    /// the given instances, with a method of one of the given lists.
    fn decorated_by(
        &self,
        definition: &Definition,
        module: usize,
        instances: &Instances,
        methods: &[&[&str]],
    ) -> bool {
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

    /// Get the groups of commands of a CLI, such as Click's: the given groups, with the functions that
    /// a decorator makes into a group, as `makes_group` tells from the groups found so far, until no
    /// more are found.
    fn groups(
        &self,
        mut groups: Instances,
        makes_group: impl Fn(&Definition, usize, &Instances) -> bool,
    ) -> Instances {
        loop {
            let mut found = Instances::default();
            for (module, file) in self.files.iter().enumerate() {
                for definition in file.definitions {
                    if definition.kind != Kind::Function
                        || !makes_group(definition, module, &groups)
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

    /// Get the name in its module of the class that a definition of a module is in.
    #[inline]
    fn class_name(&self, definition: &Definition, module: usize) -> Option<&'a str> {
        let class = definition.class?;
        Some(&self.files[module].definitions[class.get()].qualified_name)
    }

    /// Get the file of a module.
    #[inline]
    fn path(&self, module: usize) -> &'a Path {
        self.files[module].path
    }

    /// Check whether a class of a module inherits from one of the given classes, by their dotted
    /// names.
    #[inline]
    fn inherits(&self, module: usize, class: &str, bases: &[&str]) -> bool {
        self.hierarchy.inherits(module, class, bases)
    }

    /// Check whether a class of a module inherits from a class of a package whose name ends with one of
    /// the given ends.
    #[inline]
    fn inherits_end(&self, module: usize, class: &str, package: &str, ends: &[&str]) -> bool {
        has_end(&self.hierarchy.ancestors(module, class), package, ends)
    }

    /// Check whether a variable or attribute of a module is first assigned the result of calling a
    /// class or function of a package whose name ends with one of the given ends, or a subclass of
    /// such a class.
    #[inline]
    fn calls_end(
        &self,
        definition: &Definition,
        module: usize,
        package: &str,
        ends: &[&str],
    ) -> bool {
        definition
            .call
            .as_ref()
            .is_some_and(|call| has_end(&self.hierarchy.resolve(module, call), package, ends))
    }

    /// Get the attributes of the class that a definition of a module is in.
    fn class_attributes(
        &self,
        definition: &Definition,
        module: usize,
    ) -> impl Iterator<Item = &'a Definition> {
        let class = definition.class;
        self.files[module].definitions.iter().filter(move |other| {
            other.kind == Kind::Attribute && class.is_some() && other.class == class
        })
    }

    /// Check whether an expression of a module resolves to one of the given dotted names.
    #[inline]
    fn resolves_to(&self, module: usize, expression: &str, names: &[&str]) -> bool {
        self.hierarchy.resolves_to(module, expression, names)
    }

    /// Check whether a definition of a module has a decorator that resolves to one of the given
    /// dotted names.
    #[inline]
    fn is_decorated(&self, definition: &Definition, module: usize, names: &[&str]) -> bool {
        definition
            .decorators
            .iter()
            .any(|decorator| self.resolves_to(module, decorator, names))
    }

    /// Get the dotted name of a class or variable of a module, as it is among the base classes of a
    /// class.
    #[inline]
    fn dotted_name(&self, module: usize, name: &str) -> String {
        format!("{}.{name}", self.module_names[module].name)
    }

    /// Check whether an attribute of a module is annotated, and not as a class variable.
    #[inline]
    fn is_field(&self, definition: &Definition, module: usize) -> bool {
        definition
            .annotation
            .as_ref()
            .is_some_and(|annotation| !self.resolves_to(module, annotation, &CLASS_VARIABLES))
    }

    /// Find the parameters of the functions of the project that are annotated with one of the given
    /// classes, or with a subclass of one of them, given by their dotted names.
    fn annotated_parameters(&self, classes: &[&str]) -> FxHashSet<(usize, Index, String)> {
        let mut parameters = FxHashSet::default();
        for (module, file) in self.files.iter().enumerate() {
            for ((function, name), annotation) in &file.hints.annotated_parameters {
                if self.hierarchy.resolves_to(module, annotation, classes) {
                    parameters.insert((module, *function, name.clone()));
                }
            }
        }
        parameters
    }

    /// Find the local variables of the functions of the project that are set to an instance of one of
    /// the given classes, or of a subclass of one of them, given by their dotted names.
    fn local_instances(&self, classes: &[&str]) -> FxHashSet<(usize, Index, String)> {
        let mut instances = FxHashSet::default();
        for (module, file) in self.files.iter().enumerate() {
            for ((function, name), called) in &file.hints.local_calls {
                if called
                    .iter()
                    .any(|call| self.hierarchy.resolves_to(module, call, classes))
                {
                    instances.insert((module, *function, name.clone()));
                }
            }
        }
        instances
    }
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
