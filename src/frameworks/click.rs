//! Click runs the commands of a group: the functions that the `command` and `group` decorators of a
//! group register with it, and the function that its `result_callback` decorator registers. A group
//! is a variable that is set to `Group()`, or to an instance of a subclass of it, at the top level of a
//! module, or in the function that the decorated function is defined in, or a function that the
//! `group` decorator of Click, or of a group, makes into one. The same holds for the groups of
//! rich-click and asyncclick, which give out Click's API.
//!
//! A function that only Click's own `command` or `group` decorator makes into a command is not
//! registered anywhere: it runs only if something refers to it, such as an entry point or a call of
//! `add_command`, so it needs a reference.

use crate::definitions::Definition;

use super::{Call, Context, Instances, Rules, Settings, has_any};

/// The classes of the groups of commands, as `click`, `rich_click` and `asyncclick` give them out, and
/// where they are defined.
const CLICK_GROUPS: [&str; 6] = [
    "click.Group",
    "click.core.Group",
    "rich_click.RichGroup",
    "rich_click.rich_command.RichGroup",
    "asyncclick.Group",
    "asyncclick.core.Group",
];
/// The decorators that make a function into a group, as `click`, `rich_click` and `asyncclick` give
/// them out, and where they are defined.
const CLICK_GROUP_FUNCTIONS: [&str; 6] = [
    "click.group",
    "click.decorators.group",
    "rich_click.group",
    "rich_click.decorators.group",
    "asyncclick.group",
    "asyncclick.decorators.group",
];
/// The methods of a group that register the function they decorate.
const CLICK_GROUP_DECORATORS: [&str; 3] = ["command", "group", "result_callback"];

/// The rules of Click.
pub(super) struct Click {
    /// The groups of commands, as made with `Group` or with a decorator.
    groups: Instances,
}

impl Rules for Click {
    fn new(context: &Context, _settings: &Settings) -> Self {
        Self {
            groups: Instances {
                locals: context.local_instances(&CLICK_GROUPS),
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
        if has_any(call.resolved(context, module), &CLICK_GROUPS) {
            let group = context.dotted_name(module, &variable.qualified_name);
            self.groups.variables.insert(group);
        }
    }

    fn finish(&mut self, context: &Context) {
        let groups = std::mem::take(&mut self.groups);
        self.groups = context.groups(groups, |definition, module, groups| {
            context.is_decorated(definition, module, &CLICK_GROUP_FUNCTIONS)
                || context.registers(definition, module, groups, &[&["group"]])
        });
    }

    /// Check whether Click runs a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        context.registers(definition, module, &self.groups, &[&CLICK_GROUP_DECORATORS])
    }
}
