//! Typer runs the functions that the `command` and `callback` decorators of an application register:
//! of a variable that is set to `Typer()`, or to an instance of a subclass of it, at the top level of
//! a module, or in the function that the decorated function is defined in.

use crate::definitions::Definition;

use super::{Call, Context, Instances, Rules, Settings, has_any};

/// The class of Typer's applications, as `typer` gives it out, and where it is defined.
const TYPER_APPS: [&str; 2] = ["typer.Typer", "typer.main.Typer"];
/// The methods of an application that register the function they decorate.
const TYPER_DECORATORS: [&str; 2] = ["command", "callback"];

/// The rules of Typer.
pub(super) struct Typer {
    /// The applications.
    apps: Instances,
}

impl Rules for Typer {
    fn new(context: &Context, _settings: &Settings) -> Self {
        Self {
            apps: Instances {
                locals: context.local_instances(&TYPER_APPS),
                ..Instances::default()
            },
        }
    }

    fn variable_call(
        &mut self,
        context: &Context,
        module: usize,
        variable: &Definition,
        call: &Call,
    ) {
        if has_any(call.resolved(context, module), &TYPER_APPS) {
            let app = context.dotted_name(module, &variable.qualified_name);
            self.apps.variables.insert(app);
        }
    }

    /// Check whether Typer runs a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        context.registers(definition, module, &self.apps, &[&TYPER_DECORATORS])
    }
}
