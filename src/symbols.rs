//! What a module defines, at its top level or not, and imports at its top level.

use std::path::Path;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::{Definition, Kind};
use crate::modules::ModuleName;
use crate::references::{Binding, ModuleBindings, References};

/// What a module defines, at its top level or not, and imports at its top level, as the base classes
/// and decorators of its classes are followed through it, whether it is a module of the project or one outside it.
#[derive(Debug)]
pub(crate) struct ModuleSymbols {
    /// The file of the module.
    pub path: Arc<Path>,
    /// The name that the module is imported by.
    pub name: ModuleName,
    /// The names that the module imports at its top level.
    pub bindings: ModuleBindings,
    /// The modules that the module star imports at its top level, by their dotted names, sorted.
    pub stars: Vec<String>,
    /// The names that the module gives out from its `__getattr__`, each with the imports of it.
    pub lazy: FxHashMap<String, Vec<Binding>>,
    /// The classes and functions of the module, by their name in the module (`Outer.Inner` for a
    /// nested class, `function.<locals>.Local` for a class in a function), with their base classes as
    /// they are written. A function has none. Classes of one name in one scope, such as the two of an
    /// `if` and its `else`, are one class with the base classes, decorators and members of each.
    pub bases: FxHashMap<String, Vec<String>>,
    /// The classes and functions that are defined in the functions of the module, by their own name,
    /// each with its names in the module.
    pub locals: FxHashMap<String, Vec<String>>,
    /// The classes of the module, by their name in the module, with their decorators as they are
    /// written.
    pub decorators: FxHashMap<String, Vec<String>>,
    /// The names of the members of the classes of the module, by the name of the class.
    pub members: FxHashMap<String, FxHashSet<String>>,
}

impl ModuleSymbols {
    /// Collect what a module defines, and what it imports at its top level.
    pub(crate) fn of(
        path: Arc<Path>,
        name: ModuleName,
        definitions: &[Definition],
        references: &References,
        lazy: FxHashMap<String, Vec<Binding>>,
    ) -> Self {
        let mut stars: Vec<String> = references
            .imports
            .iter()
            .filter(|import| import.owner.is_none() && import.name == "*")
            .filter_map(|import| name.resolve(import.module.as_deref(), import.level))
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
            locals: FxHashMap::default(),
            decorators: FxHashMap::default(),
            members: FxHashMap::default(),
        };
        for definition in definitions {
            if matches!(definition.kind, Kind::Class | Kind::Function) {
                let name = definition.qualified_name();
                if definition.function.is_some() && !symbols.bases.contains_key(&name) {
                    symbols
                        .locals
                        .entry(definition.name.clone())
                        .or_default()
                        .push(name.clone());
                }
                symbols
                    .bases
                    .entry(name)
                    .or_default()
                    .extend(definition.bases.iter().cloned());
            }
            if definition.kind == Kind::Class {
                symbols
                    .decorators
                    .entry(definition.qualified_name())
                    .or_default()
                    .extend(definition.decorators.iter().cloned());
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
