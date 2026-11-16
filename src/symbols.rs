//! What a module defines and imports at its top level.

use std::path::Path;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::{Definition, Kind};
use crate::modules::ModuleName;
use crate::references::{Binding, References};

/// What a module defines and imports at its top level, as the base classes and decorators of its
/// classes are followed through it, whether it is a module of the project or one outside it.
#[derive(Debug)]
pub(crate) struct ModuleSymbols {
    /// The file of the module.
    pub path: Arc<Path>,
    /// The name that the module is imported by.
    pub name: ModuleName,
    /// The names that the module imports at its top level.
    pub bindings: FxHashMap<String, Binding>,
    /// The modules that the module star imports at its top level, each as it is written and with its
    /// number of leading dots.
    pub stars: Vec<(Option<String>, u32)>,
    /// The names that the module gives out from its `__getattr__`, each with the imports of it.
    pub lazy: FxHashMap<String, Vec<Binding>>,
    /// The classes and functions of the module, by their name in the module (`Outer.Inner` for a
    /// nested class), with their base classes as they are written. A function has none.
    pub bases: FxHashMap<String, Vec<String>>,
    /// The classes of the module, by their name in the module, with their decorators as they are
    /// written.
    pub decorators: FxHashMap<String, Vec<String>>,
    /// The names of the members of the classes of the module, by the name of the class.
    pub members: FxHashMap<String, FxHashSet<String>>,
}

impl ModuleSymbols {
    /// Collect what a module defines and imports at its top level.
    pub(crate) fn of(
        path: Arc<Path>,
        name: ModuleName,
        definitions: &[Definition],
        references: &References,
        lazy: FxHashMap<String, Vec<Binding>>,
    ) -> Self {
        let mut stars: Vec<(Option<String>, u32)> = references
            .imports
            .iter()
            .filter(|import| import.owner.is_none() && import.name == "*")
            .map(|import| (import.module.clone(), import.level))
            .collect();
        stars.sort();
        stars.dedup();

        let mut symbols = Self {
            path,
            name,
            bindings: references.bindings.clone(),
            stars,
            lazy,
            bases: FxHashMap::default(),
            decorators: FxHashMap::default(),
            members: FxHashMap::default(),
        };
        for definition in definitions {
            if matches!(definition.kind, Kind::Class | Kind::Function) {
                symbols
                    .bases
                    .insert(definition.qualified_name(), definition.bases.clone());
            }
            if definition.kind == Kind::Class {
                symbols
                    .decorators
                    .insert(definition.qualified_name(), definition.decorators.clone());
            }
            if let Some(class) = &definition.class {
                symbols
                    .members
                    .entry(class.clone())
                    .or_default()
                    .insert(definition.name.clone());
            }
        }
        symbols
    }
}
