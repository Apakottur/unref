//! The classes of a project with their members, to match a member read from an object of a class, and
//! the TypedDicts and enums among them.

use std::cell::OnceCell;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::classes::ClassHierarchy;
use crate::definitions::Kind;
use crate::resolver::join;

use super::{DefinitionId, Project};

/// `Protocol`, as `typing` and `typing_extensions` give it out, whose members a class has without
/// inheriting from it.
const PROTOCOLS: [&str; 2] = ["typing.Protocol", "typing_extensions.Protocol"];
const ABSTRACT_METHOD: &str = "abc.abstractmethod";
/// `TypedDict`, as `typing` and `typing_extensions` give it out.
const TYPED_DICTS: [&str; 2] = ["typing.TypedDict", "typing_extensions.TypedDict"];
/// `Enum`, which `IntEnum`, `StrEnum`, `Flag` and the other enums of `enum` inherit from.
const ENUMS: [&str; 1] = ["enum.Enum"];

/// A class of the project, by its index among the classes.
pub(super) type ClassId = usize;

/// The classes of the project, with their members, so that a member read from an object whose class
/// is plain is matched in the classes that the object can be an instance of.
///
/// The fields of a TypedDict are read as keys rather than as attributes, and the members of an enum
/// are all used by a read of the enum as a whole, so the attributes of those classes are kept too.
pub(super) struct Classes<'a> {
    pub(super) hierarchy: &'a ClassHierarchy,
    /// The class of each definition of a class, by its id, or `NONE` for another definition. Classes
    /// of one name in a module are one class.
    ids: Vec<u32>,
    /// The definition of each class, the first of its name in its module.
    pub(super) definitions: Vec<DefinitionId>,
    /// The classes by their dotted names, such as `package.module.Class`.
    by_dotted: FxHashMap<String, Vec<ClassId>>,
    /// The base classes of each class in the project, directly or not.
    ancestors: Vec<Vec<ClassId>>,
    /// The classes of the project that inherit from each class, directly or not.
    descendants: Vec<Vec<ClassId>>,
    /// Whether each class is a metaclass, whose instances are classes of their own.
    metaclasses: Vec<bool>,
    /// Whether each class inherits from `TypedDict`, directly or not.
    typed_dicts: Vec<bool>,
    /// Whether each class inherits from `Enum`, directly or not.
    enums: Vec<bool>,
    /// The attributes of each TypedDict and enum, and none for another class.
    attributes: Vec<Vec<DefinitionId>>,
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
    const NONE: u32 = u32::MAX;

    pub(super) fn new(project: &Project, hierarchy: &'a ClassHierarchy) -> Self {
        let mut ids = vec![Self::NONE; project.definitions.len()];
        let mut definitions = Vec::new();
        let mut by_dotted: FxHashMap<String, Vec<ClassId>> = FxHashMap::default();
        for (definition_id, file, definition) in project.definitions_by_file() {
            if definition.kind != Kind::Class {
                continue;
            }
            // A class merged into one before it has the id of that one.
            if let Some(first) = definition.merged_into {
                ids[definition_id] = ids[project.id(file, first)];
                continue;
            }
            let id = definitions.len();
            ids[definition_id] = u32::try_from(id).expect("fewer classes than u32::MAX");
            definitions.push(definition_id);
            let dotted = join(&project.names[file].name, &definition.qualified_name);
            by_dotted.entry(dotted).or_default().push(id);
        }
        let mut ancestors = Vec::new();
        let mut metaclasses = Vec::new();
        let mut typed_dicts = Vec::new();
        let mut enums = Vec::new();
        let mut protocols = vec![false; definitions.len()];
        let mut unknown_bases = vec![false; definitions.len()];
        for (definition_id, file, definition) in project.definitions_by_file() {
            if definition.kind != Kind::Class {
                continue;
            }
            // The loop above gave an id to every class.
            let id = ids[definition_id] as usize;
            let name = &definition.qualified_name;
            unknown_bases[id] |= definition.other_bases;
            for base in &definition.bases {
                let names = hierarchy.resolve_in(file, name, base);
                protocols[id] |= PROTOCOLS.iter().any(|protocol| names.contains(*protocol));
                unknown_bases[id] |= is_unknown_class(project, &by_dotted, file, base, &names);
            }
        }
        for &definition_id in &definitions {
            let file = project.definition_files[definition_id];
            let dotted =
                hierarchy.ancestors(file, &project.definitions[definition_id].qualified_name);
            metaclasses.push(dotted.contains("type"));
            typed_dicts.push(TYPED_DICTS.iter().any(|base| dotted.contains(*base)));
            enums.push(ENUMS.iter().any(|base| dotted.contains(*base)));
            let mut found: Vec<ClassId> = dotted
                .iter()
                .flat_map(|ancestor| by_dotted.get(ancestor).into_iter().flatten().copied())
                .collect();
            found.sort_unstable();
            found.dedup();
            ancestors.push(found);
        }
        let mut descendants = vec![Vec::new(); definitions.len()];
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
        let mut attributes = vec![Vec::new(); definitions.len()];
        for (id, definition) in project.definitions.iter().enumerate() {
            if let Some(class) = project.class_of(id) {
                let class = ids[class] as usize;
                if definition.kind == Kind::Attribute && (typed_dicts[class] || enums[class]) {
                    attributes[class].push(id);
                    // A field of a TypedDict is read as a key instead.
                    if typed_dicts[class] {
                        continue;
                    }
                }
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
            typed_dicts,
            enums,
            attributes,
            by_member,
            loose,
        }
    }

    /// Check whether a definition is a field of a TypedDict.
    pub(super) fn is_field(&self, project: &Project, id: DefinitionId) -> bool {
        project.definitions[id].kind == Kind::Attribute
            && project
                .class_of(id)
                .is_some_and(|class| self.typed_dicts[self.ids[class] as usize])
    }

    /// Get the TypedDicts that have fields, each with its definition and its fields.
    pub(super) fn typed_dicts(&self) -> impl Iterator<Item = (DefinitionId, &[DefinitionId])> {
        self.with_attributes(&self.typed_dicts)
    }

    /// Get the enums that have members, each with its definition and its members.
    pub(super) fn enums(&self) -> impl Iterator<Item = (DefinitionId, &[DefinitionId])> {
        self.with_attributes(&self.enums)
    }

    /// Get the fields of the TypedDicts of a dotted name, such as `package.module.Event`.
    pub(super) fn fields_of(&self, dotted: &str) -> impl Iterator<Item = DefinitionId> {
        self.by_dotted
            .get(dotted)
            .into_iter()
            .flatten()
            .filter(|&&class| self.typed_dicts[class])
            .flat_map(|&class| self.attributes[class].iter().copied())
    }

    /// Get the classes of a kind that have attributes, each with its definition and its attributes.
    fn with_attributes<'s>(
        &'s self,
        kind: &'s [bool],
    ) -> impl Iterator<Item = (DefinitionId, &'s [DefinitionId])> {
        kind.iter()
            .enumerate()
            .filter(|&(class, &is_kind)| is_kind && !self.attributes[class].is_empty())
            .map(|(class, _)| (self.definitions[class], &*self.attributes[class]))
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
        class: DefinitionId,
        name: &str,
        skip_own: bool,
    ) -> Option<Vec<DefinitionId>> {
        let id = self.ids[class] as usize;
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
            let Some(class) = project.class_of(id) else {
                continue;
            };
            if !definition.kind.is_function() || definition.decorators.is_empty() {
                continue;
            }
            let Some(by_class) = self.by_member.get(&definition.name) else {
                continue;
            };
            let class = self.ids[class] as usize;
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
