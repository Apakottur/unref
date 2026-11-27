//! Pydantic reads the fields of the subclasses of `pydantic.BaseModel`, the names they annotate
//! other than class variables and private attributes, and their `model_config`, or their `Config`
//! class in Pydantic 1. It calls the methods that its validator and serializer decorators register,
//! and reads the properties that `computed_field` registers.

use crate::definitions::{Definition, Kind};

use super::{Context, Rules, Settings};

/// `pydantic.BaseModel` as `pydantic` gives it out, and where it is defined, also in the Pydantic 1 API
/// of `pydantic.v1`. Packages such as `pydantic_settings` import it from where it is defined.
pub(super) const PYDANTIC_MODELS: [&str; 4] = [
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

/// The rules of Pydantic.
pub(super) struct Pydantic;

impl Rules for Pydantic {
    fn new(_context: &Context, _settings: &Settings) -> Self {
        Self
    }

    /// Check whether Pydantic calls or reads a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        let Some(class) = context.class_name(definition, module) else {
            return false;
        };
        if definition.kind == Kind::Method {
            // A decorator registers the method with Pydantic, in a model or a Pydantic dataclass.
            return context.is_decorated(definition, module, &PYDANTIC_DECORATORS);
        }
        if definition.kind == Kind::Class {
            return definition.name == PYDANTIC_V1_CONFIG && is_model(context, module, class);
        }

        // An attribute, which a decorator applied by hand can register as it does a method.
        if definition
            .wrapped_by
            .as_ref()
            .is_some_and(|decorator| context.resolves_to(module, decorator, &PYDANTIC_DECORATORS))
        {
            return true;
        }
        let (model, last) = class.rsplit_once('.').unwrap_or(("", class));
        if last == PYDANTIC_V1_CONFIG && !model.is_empty() && is_model(context, module, model) {
            return true;
        }
        is_model(context, module, class)
            && (definition.name == PYDANTIC_CONFIG
                // A field that is not a private attribute.
                || (!definition.name.starts_with('_') && context.is_field(definition, module)))
    }
}

/// Check whether a class of a module is a subclass of `pydantic.BaseModel`.
fn is_model(context: &Context, module: usize, name: &str) -> bool {
    context.inherits(module, name, &PYDANTIC_MODELS)
}
