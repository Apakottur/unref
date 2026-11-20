//! The dataclasses of `dataclasses`, Pydantic and attrs, and the subclasses of `typing.NamedTuple`,
//! read their fields in the methods that they generate, such as `__eq__` and `__repr__`, and in
//! functions such as `dataclasses.asdict`. A field is a name that the class annotates other than a
//! class variable, or, in a class of attrs, a name that is set to a call of `attr.ib` or
//! `attrs.field`. A class is a dataclass when it has one of their decorators itself, so a subclass
//! of a dataclass without one has no fields of its own.

use crate::definitions::{Definition, Kind};

use super::{FrameworkRules, has_any};

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

impl FrameworkRules<'_> {
    /// Check whether a dataclass, a named tuple or a class of attrs reads a definition by itself: one
    /// of its fields.
    pub(super) fn dataclasses_uses(&self, definition: &Definition, module: usize) -> bool {
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
}
