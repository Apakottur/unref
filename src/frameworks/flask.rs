//! Flask calls the functions that the decorators of an application or a blueprint register, such as
//! `app.route` or `bp.errorhandler`: of a variable that is set to `Flask()` or `Blueprint()`, or to
//! an instance of a subclass of them, at the top level of a module, or in the function that the
//! decorated function is defined in, of a parameter of that function that is annotated as one, or
//! that pytest passes a fixture to that returns or yields one, of a variable at the top level that
//! is set to what a factory makes, and of `current_app`. It calls the methods of a `MethodView`
//! named after HTTP methods, and the `dispatch_request` method of a `View`. Its CLI runs the
//! commands that `cli.command` and `cli.group` of an application or a blueprint add, and those that
//! `command` and `group` of a group add, a group being an instance of `AppGroup` or a function that
//! such a decorator makes one. It calls a factory, a function at the top level named `create_app`
//! or `make_app` that sets a local variable to an application, and loads a variable at the top
//! level named `app` or `application` that is set to an application or to what a factory makes.

use rustc_hash::FxHashSet;

use crate::definitions::{Definition, Kind};
use crate::references::Index;

use super::pytest::{PYTEST_FIXTURES, Pytest};
use super::{Call, Context, HTTP_METHODS, Instances, Rules, Settings, has_any, is_top_level};

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

/// The rules of Flask.
pub(super) struct Flask {
    /// The applications and blueprints.
    apps: Instances,
    /// The groups of commands of the CLI, as made with `AppGroup` or with a decorator.
    groups: Instances,
    /// The factories of applications that the CLI finds, by their dotted names.
    factories: FxHashSet<String>,
}

impl Rules for Flask {
    fn new(context: &Context, _settings: &Settings) -> Self {
        let mut flask = Self {
            apps: Instances::default(),
            groups: Instances::default(),
            factories: factories(context),
        };
        flask.apps.locals = context.local_instances(&FLASK_APPS);
        flask
            .apps
            .locals
            .extend(context.annotated_parameters(&FLASK_APPS));
        flask
            .apps
            .variables
            .extend(FLASK_CURRENT_APP.map(String::from));
        let parameters = flask.fixture_parameters(context);
        flask.apps.locals.extend(parameters);
        flask.groups.locals = context.local_instances(&FLASK_GROUPS);
        flask
    }

    fn variable_call(
        &mut self,
        context: &Context,
        module: usize,
        variable: &Definition,
        call: &Call,
    ) {
        let called = call.resolved(context, module);
        let dotted = context.dotted_name(module, &variable.qualified_name);
        if has_any(called, &FLASK_APPS) || self.is_factory(context, module, call.written) {
            self.apps.variables.insert(dotted);
        } else if has_any(called, &FLASK_GROUPS) {
            self.groups.variables.insert(dotted);
        }
    }

    fn finish(&mut self, context: &Context) {
        self.groups = self.command_groups(context);
    }

    /// Check whether Flask calls a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        if definition.kind == Kind::Method
            && let Some(class) = context.class_name(definition, module)
        {
            let name = definition.name.as_str();
            if (HTTP_METHODS.contains(&name)
                && context.inherits(module, class, &[FLASK_METHOD_VIEW]))
                || (name == "dispatch_request" && context.inherits(module, class, &FLASK_VIEWS))
            {
                return true;
            }
        }
        if is_top_level(definition) {
            let name = definition.name.as_str();
            if definition.kind == Kind::Function && FLASK_FACTORIES.contains(&name) {
                return self.factories.contains(&context.dotted_name(module, name));
            }
            if definition.kind == Kind::Variable && FLASK_APP_VARIABLES.contains(&name) {
                return definition.call.as_ref().is_some_and(|call| {
                    context.resolves_to(module, call, &FLASK_APPLICATIONS)
                        || self.is_factory(context, module, call)
                });
            }
        }
        context.registers(
            definition,
            module,
            &self.apps,
            &[&FLASK_DECORATORS, &FLASK_CLI_DECORATORS],
        ) || context.registers(definition, module, &self.groups, &[&FLASK_GROUP_DECORATORS])
    }
}

impl Flask {
    /// Find the parameters that pytest passes an application of Flask to: those of a test or a fixture
    /// that are named as a fixture that returns or yields an application, or what a factory makes.
    ///
    /// The tests and the fixtures are those that pytest's rules find, whether they apply or not, and
    /// without the base classes of test classes, which pytest finds in the pass over the definitions,
    /// after this.
    fn fixture_parameters(&self, context: &Context) -> FxHashSet<(usize, Index, String)> {
        let pytest = Pytest::default();
        let mut fixtures: FxHashSet<&str> = FxHashSet::default();
        // The tests and the fixtures, which pytest passes fixtures to, by their module and index.
        let mut tests_and_fixtures: FxHashSet<(usize, Index)> = FxHashSet::default();
        for (module, file) in context.files.iter().enumerate() {
            for (index, definition) in file.definitions.iter().enumerate() {
                let index = Index::new(index);
                if definition.kind.is_function() && pytest.uses(context, definition, module) {
                    tests_and_fixtures.insert((module, index));
                }
                if definition.kind == Kind::Function
                    && context.is_decorated(definition, module, &PYTEST_FIXTURES)
                    && file.hints.returned_calls.get(&index).is_some_and(|calls| {
                        calls.iter().any(|call| {
                            context.resolves_to(module, call, &FLASK_APPS)
                                || self.is_factory(context, module, call)
                        })
                    })
                {
                    fixtures.insert(&definition.name);
                }
            }
        }
        let mut parameters = FxHashSet::default();
        for (module, file) in context.files.iter().enumerate() {
            for (function, name) in &file.hints.parameters {
                if fixtures.contains(name.as_str())
                    && tests_and_fixtures.contains(&(module, *function))
                {
                    parameters.insert((module, *function, name.clone()));
                }
            }
        }
        parameters
    }

    /// Check whether a call of a module calls a factory of Flask's applications.
    fn is_factory(&self, context: &Context, module: usize, call: &str) -> bool {
        self.factories.contains(&context.dotted_name(module, call))
            || context
                .hierarchy
                .resolve(module, call)
                .iter()
                .any(|name| self.factories.contains(name))
    }

    /// Get the groups of commands of Flask's CLI: the instances of `AppGroup`, and the functions that
    /// the `cli.group` decorator of an application or a blueprint, or the `group` decorator of a group,
    /// makes into a group.
    fn command_groups(&self, context: &Context) -> Instances {
        context.groups(self.groups.clone(), |definition, module, groups| {
            context.registers(definition, module, &self.apps, &[&["cli.group"]])
                || context.registers(definition, module, groups, &[&["group"]])
        })
    }
}

/// Find the factories of Flask's applications that the Flask CLI finds: the functions at the top level
/// of a module that are named as one, and set a local variable to an application.
fn factories(context: &Context) -> FxHashSet<String> {
    let applications: FxHashSet<(usize, Index)> = context
        .local_instances(&FLASK_APPLICATIONS)
        .into_iter()
        .map(|(module, function, _)| (module, function))
        .collect();
    let mut factories = FxHashSet::default();
    for (module, file) in context.files.iter().enumerate() {
        for (index, definition) in file.definitions.iter().enumerate() {
            if definition.kind == Kind::Function
                && is_top_level(definition)
                && FLASK_FACTORIES.contains(&definition.name.as_str())
                && applications.contains(&(module, Index::new(index)))
            {
                factories.insert(context.dotted_name(module, &definition.name));
            }
        }
    }
    factories
}
