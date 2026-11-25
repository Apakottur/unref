//! Strawberry builds a GraphQL schema from the classes that its decorators make into types, whose
//! fields it reads, and calls the resolvers that its field decorators register. An enum that its
//! decorator registers has all its members in the schema.

use crate::definitions::{Definition, Kind};

use super::{Context, Rules, Settings, has_any};

/// The decorators that make a class into a type of the schema, whose fields Strawberry reads.
const STRAWBERRY_TYPES: [&str; 16] = [
    "strawberry.type",
    "strawberry.input",
    "strawberry.interface",
    "strawberry.schema_directive",
    "strawberry.federation.type",
    "strawberry.federation.input",
    "strawberry.federation.interface",
    "strawberry.federation.interface_object",
    "strawberry.federation.schema_directive",
    "strawberry.pydantic.type",
    "strawberry.pydantic.input",
    "strawberry.pydantic.interface",
    "strawberry.experimental.pydantic.type",
    "strawberry.experimental.pydantic.input",
    "strawberry.experimental.pydantic.interface",
    "strawberry.experimental.pydantic.error_type",
];
/// The decorators that register a method as a resolver of a field.
const STRAWBERRY_RESOLVERS: [&str; 8] = [
    "strawberry.field",
    "strawberry.mutation",
    "strawberry.subscription",
    "strawberry.federation.field",
    "strawberry.federation.mutation",
    "strawberry.relay.connection",
    "strawberry.relay.node",
    "strawberry.pydantic.field",
];
/// The decorators that register an enum, with all its members.
const STRAWBERRY_ENUMS: [&str; 2] = ["strawberry.enum", "strawberry.federation.enum"];

/// The rules of Strawberry.
pub(super) struct Strawberry;

impl Rules for Strawberry {
    fn new(_context: &Context, _settings: &Settings) -> Self {
        Self
    }

    /// Check whether Strawberry reads or calls a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        let Some(class) = context.class_name(definition, module) else {
            return false;
        };
        match definition.kind {
            Kind::Method => context.is_decorated(definition, module, &STRAWBERRY_RESOLVERS),
            Kind::Attribute => {
                let decorators = context.hierarchy.decorators(module, class);
                has_any(&decorators, &STRAWBERRY_ENUMS)
                    || (has_any(&decorators, &STRAWBERRY_TYPES)
                        && context.is_field(definition, module))
            }
            _ => false,
        }
    }
}
