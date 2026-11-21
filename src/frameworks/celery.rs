//! Celery's workers run the tasks that `shared_task` registers, and those that the `task` decorator
//! of an application registers: of a variable that is set to `Celery()`, or to an instance of a
//! subclass of it, at the top level of a module, or in the function that the decorated function is
//! defined in, of a parameter of that function that is annotated as one, and of `current_app`. A
//! task runs by its name, such as from the schedule of Celery beat, so it needs no reference.
//! Celery calls the `run` method and the handlers, such as `on_failure`, of the subclasses of
//! `Task`.

use crate::definitions::{Definition, Kind};

use super::{Call, Context, Instances, Rules, Settings, has_any};

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

/// The rules of Celery.
pub(super) struct Celery {
    /// The applications.
    apps: Instances,
}

impl Rules for Celery {
    fn new(context: &Context, _settings: &Settings) -> Self {
        let mut locals = context.local_instances(&CELERY_APPS);
        locals.extend(context.annotated_parameters(&CELERY_APPS));
        Self {
            apps: Instances {
                variables: CELERY_CURRENT_APP.map(String::from).into_iter().collect(),
                locals,
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
        if has_any(call.resolved(context, module), &CELERY_APPS) {
            let app = context.dotted_name(module, &variable.qualified_name);
            self.apps.variables.insert(app);
        }
    }

    /// Check whether Celery runs or calls a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        if definition.kind == Kind::Method
            && let Some(class) = context.class_name(definition, module)
            && CELERY_TASK_METHODS.contains(&definition.name.as_str())
            && context.inherits(module, class, &CELERY_TASKS)
        {
            return true;
        }
        (definition.kind.is_function()
            && context.is_decorated(definition, module, &CELERY_SHARED_TASKS))
            || context.registers(definition, module, &self.apps, &[&CELERY_TASK_DECORATORS])
    }
}
