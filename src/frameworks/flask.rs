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

use super::pytest::PYTEST_FIXTURES;
use super::{FrameworkRules, HTTP_METHODS, Instances, ProjectFile, is_top_level, local_instances};

/// The classes of Flask's applications and blueprints, as `flask` gives them out, and where they are
/// defined.
pub(super) const FLASK_APPS: [&str; 4] = [
    FLASK_APPLICATIONS[0],
    FLASK_APPLICATIONS[1],
    "flask.Blueprint",
    "flask.blueprints.Blueprint",
];
pub(super) const FLASK_APPLICATIONS: [&str; 2] = ["flask.Flask", "flask.app.Flask"];
/// `current_app`, which stands for the application, as `flask` gives it out, and where it is defined.
pub(super) const FLASK_CURRENT_APP: [&str; 2] = ["flask.current_app", "flask.globals.current_app"];
/// The factories and the variables of an application that the Flask CLI finds by their names.
pub(super) const FLASK_FACTORIES: [&str; 2] = ["create_app", "make_app"];
pub(super) const FLASK_APP_VARIABLES: [&str; 2] = ["app", "application"];
/// The methods of an application or a blueprint that register the function they decorate.
pub(super) const FLASK_DECORATORS: [&str; 33] = [
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
pub(super) const FLASK_CLI_DECORATORS: [&str; 2] = ["cli.command", "cli.group"];
/// The classes of the groups of commands of Flask's CLI, as `flask.cli` gives them out, with the
/// methods that add a command or a group to a group.
pub(super) const FLASK_GROUPS: [&str; 2] = ["flask.cli.AppGroup", "flask.cli.FlaskGroup"];
pub(super) const FLASK_GROUP_DECORATORS: [&str; 2] = ["command", "group"];
/// The class views of Flask, whose `dispatch_request` method it calls. `MethodView` inherits from
/// `View`, and its `dispatch_request` calls its methods named after HTTP methods.
pub(super) const FLASK_VIEWS: [&str; 2] = ["flask.views.View", FLASK_METHOD_VIEW];
pub(super) const FLASK_METHOD_VIEW: &str = "flask.views.MethodView";

impl FrameworkRules<'_> {
    /// Check whether Flask calls a definition by itself.
    pub(super) fn flask_uses(&self, definition: &Definition, module: usize) -> bool {
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
    pub(super) fn flask_factories(&self, files: &[ProjectFile]) -> FxHashSet<String> {
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

    /// Find the parameters that pytest passes an application of Flask to: those of a test or a fixture
    /// that are named as a fixture that returns or yields an application, or what a factory makes.
    pub(super) fn flask_fixture_parameters(
        &self,
        files: &[ProjectFile],
    ) -> FxHashSet<(usize, u32, String)> {
        let mut fixtures: FxHashSet<&str> = FxHashSet::default();
        // The tests and the fixtures, which pytest passes fixtures to, by their module and line.
        let mut tests_and_fixtures: FxHashSet<(usize, u32)> = FxHashSet::default();
        for (module, (_, definitions, references)) in files.iter().enumerate() {
            for definition in *definitions {
                if definition.kind.is_function() && self.pytest_uses(definition, module) {
                    tests_and_fixtures.insert((module, definition.line));
                }
                if definition.kind == Kind::Function
                    && self.is_decorated(definition, module, &PYTEST_FIXTURES)
                    && references
                        .returned_calls
                        .get(&definition.line)
                        .is_some_and(|calls| {
                            calls.iter().any(|call| {
                                self.resolves_to(module, call, &FLASK_APPS)
                                    || self.is_flask_factory(module, call)
                            })
                        })
                {
                    fixtures.insert(&definition.name);
                }
            }
        }
        let mut parameters = FxHashSet::default();
        for (module, (_, _, references)) in files.iter().enumerate() {
            for (function, name) in &references.parameters {
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
    pub(super) fn is_flask_factory(&self, module: usize, call: &str) -> bool {
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
    pub(super) fn flask_command_groups(&self, files: &[ProjectFile]) -> Instances {
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
}
