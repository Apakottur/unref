//! The classes of a project with their members, to match a member read from an object of a class.

use std::cell::OnceCell;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::classes::{ClassHierarchy, join};
use crate::definitions::Kind;

use super::{DefinitionId, Project};

/// `Protocol`, as `typing` and `typing_extensions` give it out, whose members a class has without
/// inheriting from it.
const PROTOCOLS: [&str; 2] = ["typing.Protocol", "typing_extensions.Protocol"];
const ABSTRACT_METHOD: &str = "abc.abstractmethod";

/// A class of the project, by its index among the classes.
pub(super) type ClassId = usize;

/// The classes of the project, with their members, so that a member read from an object whose class
/// is plain is matched in the classes that the object can be an instance of.
pub(super) struct Classes<'a> {
    pub(super) hierarchy: &'a ClassHierarchy,
    /// Each class by its file and its name in the module.
    ids: FxHashMap<(usize, String), ClassId>,
    /// The definition of each class.
    pub(super) definitions: Vec<DefinitionId>,
    /// The classes by their dotted names, such as `package.module.Class`.
    by_dotted: FxHashMap<String, Vec<ClassId>>,
    /// The base classes of each class in the project, directly or not.
    ancestors: Vec<Vec<ClassId>>,
    /// The classes of the project that inherit from each class, directly or not.
    descendants: Vec<Vec<ClassId>>,
    /// Whether each class is a metaclass, whose instances are classes of their own.
    metaclasses: Vec<bool>,
    /// The classes that a member read from the first parameter of a method of each class can be in,
    /// and from `super()` in it, found when first needed.
    lineages: Vec<[OnceCell<Vec<ClassId>>; 2]>,
    /// The members of the classes, by their name and then their class. A field of a `TypedDict` is
    /// read as a key instead.
    by_member: FxHashMap<String, FxHashMap<ClassId, Vec<DefinitionId>>>,
    /// The members, by their name, of the classes that any member read can stand for: the
    /// protocols, which a class matches without inheriting from them, and the classes that inherit
    /// from a base class that is not known, such as a call or a variable, and the classes that
    /// inherit from those.
    loose: FxHashMap<String, Vec<DefinitionId>>,
}

impl<'a> Classes<'a> {
    pub(super) fn new(
        project: &Project,
        hierarchy: &'a ClassHierarchy,
        fields: &FxHashSet<DefinitionId>,
    ) -> Self {
        let mut keys: Vec<(usize, String)> = Vec::new();
        let mut ids = FxHashMap::default();
        let mut definitions = Vec::new();
        let mut by_dotted: FxHashMap<String, Vec<ClassId>> = FxHashMap::default();
        for (definition_id, file, definition) in project.definitions_by_file() {
            if definition.kind == Kind::Class {
                let key = (file, definition.qualified_name());
                if ids.contains_key(&key) {
                    continue;
                }
                let id = keys.len();
                ids.insert(key.clone(), id);
                definitions.push(definition_id);
                let dotted = join(&project.names[file].name, &key.1);
                by_dotted.entry(dotted).or_default().push(id);
                keys.push(key);
            }
        }
        let mut ancestors = Vec::new();
        let mut metaclasses = Vec::new();
        let mut protocols = vec![false; keys.len()];
        let mut unknown_bases = vec![false; keys.len()];
        for (_, file, definition) in project.definitions_by_file() {
            if definition.kind != Kind::Class {
                continue;
            }
            // The loop above gave an id to every class.
            let name = definition.qualified_name();
            let id = ids[&(file, name.clone())];
            unknown_bases[id] |= definition.other_bases;
            for base in &definition.bases {
                let names = hierarchy.resolve_in(file, &name, base);
                protocols[id] |= PROTOCOLS.iter().any(|protocol| names.contains(*protocol));
                unknown_bases[id] |= is_unknown_class(project, &by_dotted, file, base, &names);
            }
        }
        for (file, name) in &keys {
            let dotted = hierarchy.ancestors(*file, name);
            metaclasses.push(dotted.contains("type"));
            let mut found: Vec<ClassId> = dotted
                .iter()
                .flat_map(|ancestor| by_dotted.get(ancestor).into_iter().flatten().copied())
                .collect();
            found.sort_unstable();
            found.dedup();
            ancestors.push(found);
        }
        let mut descendants = vec![Vec::new(); keys.len()];
        for (id, found) in ancestors.iter().enumerate() {
            for &ancestor in found {
                if ancestor != id {
                    descendants[ancestor].push(id);
                }
            }
        }
        // What a class inherits from is not known if what one of its base classes inherits from is not.
        // A subclass of a protocol inherits from it as from any other class, so only the protocol
        // itself is matched by name.
        let is_loose: Vec<bool> = ancestors
            .iter()
            .enumerate()
            .map(|(id, found)| {
                protocols[id]
                    || std::iter::once(&id)
                        .chain(found)
                        .any(|&class| unknown_bases[class])
            })
            .collect();
        let mut by_member: FxHashMap<String, FxHashMap<ClassId, Vec<DefinitionId>>> =
            FxHashMap::default();
        let mut loose: FxHashMap<String, Vec<DefinitionId>> = FxHashMap::default();
        for (id, file, definition) in project.definitions_by_file() {
            if let Some(class_name) = &definition.class
                && !fields.contains(&id)
                && let Some(&class) = ids.get(&(file, class_name.clone()))
            {
                by_member
                    .entry(definition.name.clone())
                    .or_default()
                    .entry(class)
                    .or_default()
                    .push(id);
                if is_loose[class] {
                    loose.entry(definition.name.clone()).or_default().push(id);
                }
            }
        }
        Self {
            hierarchy,
            ids,
            definitions,
            by_dotted,
            ancestors,
            lineages: std::iter::repeat_with(Default::default)
                .take(descendants.len())
                .collect(),
            descendants,
            metaclasses,
            by_member,
            loose,
        }
    }

    /// Find the members of a name in classes and their base classes.
    ///
    /// Returns None if none of them has a member of that name, since the member then comes from
    /// somewhere unknown, such as a base class outside the project or a class decorator.
    pub(super) fn members(
        &self,
        classes: impl IntoIterator<Item = ClassId>,
        name: &str,
    ) -> Option<Vec<DefinitionId>> {
        let by_class = self.by_member.get(name)?;
        let mut found = Vec::new();
        for class in classes {
            if let Some(members) = by_class.get(&class) {
                found.extend(members);
            }
        }
        if found.is_empty() {
            return None;
        }
        found.extend(self.loose.get(name).into_iter().flatten());
        Some(found)
    }

    /// Find the members that a name read from a dotted name can stand for, as `name` in
    /// `package.module.Class.name` or `Class().name`: those of the class and of its base classes.
    ///
    /// Returns None if the dotted name is not a class of the project, or if the class has no such
    /// member.
    pub(super) fn read_from(&self, written: &str, name: &str) -> Option<Vec<DefinitionId>> {
        let classes: Vec<ClassId> = self
            .hierarchy
            .follow_name(written)
            .iter()
            .flat_map(|dotted| self.by_dotted.get(dotted).into_iter().flatten().copied())
            .collect();
        if classes.is_empty() {
            return None;
        }
        self.members(classes, name)
    }

    /// Find the members that a name read from the first parameter of a method of a class can stand
    /// for, or from `super()` in it.
    ///
    /// The object can be an instance of the class or of a class that inherits from it, so a member is
    /// looked for in those classes and in their base classes, as a base class can call a method that
    /// a subclass overrides, or one of another base class of a subclass. `super()` skips the class and
    /// the classes that inherit from it.
    ///
    /// Returns None if the class is a metaclass, or if none of those classes has such a member.
    pub(super) fn read_from_own(
        &self,
        file: usize,
        class: &str,
        name: &str,
        skip_own: bool,
    ) -> Option<Vec<DefinitionId>> {
        let &id = self.ids.get(&(file, class.to_string()))?;
        if self.metaclasses[id] || !self.by_member.contains_key(name) {
            return None;
        }
        let lineage = self.lineages[id][usize::from(skip_own)].get_or_init(|| {
            let mut lineage: FxHashSet<ClassId> = FxHashSet::default();
            for &instance in std::iter::once(&id).chain(&self.descendants[id]) {
                lineage.insert(instance);
                lineage.extend(&self.ancestors[instance]);
            }
            if skip_own {
                lineage.remove(&id);
                for descendant in &self.descendants[id] {
                    lineage.remove(descendant);
                }
            }
            lineage.into_iter().collect()
        });
        self.members(lineage.iter().copied(), name)
    }

    /// Find the implementations of each abstract method: the members of its name in the classes that
    /// inherit from its class, each with the definition of its class.
    pub(super) fn implementations(
        &self,
        project: &Project,
    ) -> FxHashMap<DefinitionId, Vec<(DefinitionId, DefinitionId)>> {
        let mut implementations = FxHashMap::default();
        for (id, file, definition) in project.definitions_by_file() {
            let Some(class_name) = &definition.class else {
                continue;
            };
            if !definition.kind.is_function() || definition.decorators.is_empty() {
                continue;
            }
            let (Some(&class), Some(by_class)) = (
                self.ids.get(&(file, class_name.clone())),
                self.by_member.get(&definition.name),
            ) else {
                continue;
            };
            let found: Vec<(DefinitionId, DefinitionId)> = self.descendants[class]
                .iter()
                .flat_map(|&subclass| {
                    by_class
                        .get(&subclass)
                        .into_iter()
                        .flatten()
                        .map(move |&member| (self.definitions[subclass], member))
                })
                .collect();
            if !found.is_empty()
                && definition.decorators.iter().any(|decorator| {
                    self.hierarchy
                        .resolves_to(file, decorator, &[ABSTRACT_METHOD])
                })
            {
                implementations.insert(id, found);
            }
        }
        implementations
    }
}

/// Check whether a base class of a class, as it is written in its module, is not known: it is
/// none of the classes of the project, but a variable or another name of a module of the project.
///
/// `names` are the dotted names that the base class stands for.
fn is_unknown_class(
    project: &Project,
    by_dotted: &FxHashMap<String, Vec<ClassId>>,
    file: usize,
    written: &str,
    names: &FxHashSet<String>,
) -> bool {
    if names.iter().any(|name| by_dotted.contains_key(name)) {
        return false;
    }
    names.iter().any(|name| {
        if name == written && !written.contains('.') {
            // A name that the module neither imports nor defines as a class or function, such as a
            // builtin or a variable of the module.
            return project.file_definitions(file).iter().any(|definition| {
                definition.class.is_none()
                    && definition.owner.is_none()
                    && definition.name == written
            });
        }
        project.holds(name)
    })
}
