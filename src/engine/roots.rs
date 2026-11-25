//! The definitions that count as used whatever references them, and those that are ignored.

use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::classes::ClassHierarchy;
use crate::config::Config;
use crate::definitions::Definition;
use crate::discover::is_under;
use crate::frameworks::FrameworkRules;
use crate::ignores::ignored_codes;
use crate::models::{Override, fnmatch};
use crate::modules::resolve;

use super::{DefinitionId, Project};

/// The object reference of an entry point, as the entry points specification of the Python Packaging
/// User Guide gives it.
pub(super) static OBJECT_REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<module>[\w.]+)\s*(:\s*(?P<attributes>[\w.]+)\s*)?(\[.*\]\s*)?$")
        .expect("valid pattern")
});

/// Find the definitions that count as used whatever references them, and those that are ignored.
///
/// Returns the definitions that count as used by the options or as dunders, and the other definitions
/// that an ignore comment ignores.
pub(super) fn find_roots(
    project: &Project,
    hierarchy: &ClassHierarchy,
    options: &Config,
    rules: &FrameworkRules,
) -> (Vec<DefinitionId>, Vec<DefinitionId>) {
    let dummy_variable = fancy_regex::Regex::new(&options.dummy_variable_rgx).ok();
    let used_decorators: FxHashSet<&str> =
        options.used_decorators.iter().map(String::as_str).collect();
    let overrides: Vec<(Vec<PathBuf>, &Override, FxHashSet<&str>)> = options
        .overrides
        .iter()
        .map(|item| {
            (
                item.paths.iter().map(|path| resolve(path)).collect(),
                item,
                item.used_decorators.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    let entry_objects = entry_objects(&options.entry_points, &project.files_by_name);

    let mut kept = Vec::new();
    let mut ignored = Vec::new();
    for (file, path) in project.files.iter().enumerate() {
        // An override with paths applies to the files under them.
        let file_overrides: Vec<_> = overrides
            .iter()
            .filter(|(roots, _, _)| roots.is_empty() || is_under(path, roots))
            .collect();
        let ignored_in_file = ignored_codes(&project.facts[file].ignore_comments);
        for id in project.ids_of_file(file) {
            let definition = &project.definitions[id];
            let class = project.class_name(id);
            let name = definition.name.as_str();
            let is_kept = name == "_"
                || is_dunder(name)
                // A variable that is not used on purpose, where Python needs a name to bind. Ruff
                // searches for the pattern, rather than matching the whole name.
                || (definition.unpacked
                    && dummy_variable
                        .as_ref()
                        .is_some_and(|pattern| pattern.is_match(name).unwrap_or(false)))
                // A member that a base class outside the project defines is used by its code.
                || class.is_some_and(|class| hierarchy.external_members(file, class).contains(name))
                // The objects that the entry points name.
                || entry_objects
                    .get(&file)
                    .is_some_and(|objects| objects.contains(&definition.qualified_name))
                // What the frameworks, and classes of the standard library, find and call by themselves.
                || rules.uses(definition, file)
                // The options of a definition are those of the project, plus those of every override
                // that covers it.
                || matches_options(definition, &options.used_names, &used_decorators)
                || file_overrides.iter().any(|(_, item, decorators)| {
                    covers(item, file, class, hierarchy) && matches_options(definition, &item.used_names, decorators)
                });
            if is_kept {
                kept.push(id);
            } else if ignored_in_file.contains(&(definition.line, definition.kind.code().name())) {
                ignored.push(id);
            }
        }
    }
    (kept, ignored)
}

/// Check whether an override that applies to a file covers a definition of it, given by the name in
/// the module of the class that it is in: whether the definition matches the base classes and the
/// class decorators of the override.
fn covers(item: &Override, file: usize, class: Option<&str>, hierarchy: &ClassHierarchy) -> bool {
    // An override with base classes applies to what is defined in a class that inherits from one of
    // them.
    if !item.base_classes.is_empty() {
        let inherits =
            class.is_some_and(|class| hierarchy.inherits(file, class, &item.base_classes));
        if !inherits {
            return false;
        }
    }
    // An override with class names applies to what is defined in a class whose own name, without the
    // classes it is nested in, matches one of the patterns.
    if !item.class_names.is_empty() {
        let named = class.is_some_and(|class| {
            let name = class.rsplit('.').next().unwrap_or(class);
            item.class_names
                .iter()
                .any(|pattern| fnmatch(name, pattern))
        });
        if !named {
            return false;
        }
    }
    // An override with class decorators applies to what is defined in a class that has one of them.
    item.class_decorators.is_empty()
        || class.is_some_and(|class| {
            let decorators = hierarchy.decorators(file, class);
            item.class_decorators
                .iter()
                .any(|decorator| decorators.contains(decorator))
        })
}

/// Check whether options make a definition count as used: its name matches one of the patterns, or it
/// has one of the decorators.
fn matches_options(
    definition: &Definition,
    names: &[String],
    decorators: &FxHashSet<&str>,
) -> bool {
    names
        .iter()
        .any(|pattern| fnmatch(&definition.name, pattern))
        || definition
            .decorators
            .iter()
            .any(|decorator| decorators.contains(decorator.as_str()))
}

/// Find the objects that the entry points of a project name.
///
/// Returns the qualified names of the objects in each file. `module:Class.method` names the class, and
/// the method in it if there is one.
fn entry_objects(
    entry_points: &[String],
    files_by_name: &FxHashMap<String, Vec<usize>>,
) -> FxHashMap<usize, FxHashSet<String>> {
    let mut objects: FxHashMap<usize, FxHashSet<String>> = FxHashMap::default();
    for reference in entry_points {
        let Some(found) = OBJECT_REFERENCE.captures(reference.trim()) else {
            continue;
        };
        let attributes: Vec<&str> = found
            .name("attributes")
            .map_or_default(|attributes| attributes.as_str().split('.').collect());
        for &file in files_by_name.get(&found["module"]).into_iter().flatten() {
            objects
                .entry(file)
                .or_default()
                .extend((1..=attributes.len()).map(|index| attributes[..index].join(".")));
        }
    }
    objects
}

/// Check whether a name is a dunder, such as `__init__` or `__all__`.
///
/// Python and tools refer to dunders implicitly, so they always count as used.
fn is_dunder(name: &str) -> bool {
    name.len() > 4 && name.starts_with("__") && name.ends_with("__")
}
