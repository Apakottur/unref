//! What the definitions of a project use, followed from the code of the modules.

use ruff_python_ast::name::Name;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{AllUses, Classes, CodeId, DefinitionId, Project, Uses};

/// The definitions of a project and what the code of each module and function uses.
///
/// The code of a module runs when it is imported, and the code of a function only when it is called.
/// So a function that is unused uses nothing.
pub(super) struct Graph<'a> {
    project: &'a Project,
    uses: &'a AllUses,
    /// The members of each enum of the project, by its file and its name in the module, with their
    /// names.
    enums: FxHashMap<(usize, String), (Vec<DefinitionId>, FxHashSet<String>)>,
    by_name: FxHashMap<(usize, Name), Vec<DefinitionId>>,
    /// The names of each module, as `by_name` holds them, for the modules that are read as objects.
    module_names: FxHashMap<usize, Vec<(Name, DefinitionId)>>,
    /// The definitions that `by_name` holds, by their name only, for the names read from a module
    /// that is imported by a string.
    module_level: FxHashMap<Name, Vec<DefinitionId>>,
    by_attribute: FxHashMap<String, Vec<DefinitionId>>,
    by_key: FxHashMap<String, Vec<DefinitionId>>,
    public: FxHashMap<usize, Vec<DefinitionId>>,
    /// The implementations of each abstract method, each with the definition of its class. An
    /// implementation is used once both the abstract method and its class are.
    implementations: FxHashMap<DefinitionId, Vec<(DefinitionId, DefinitionId)>>,
    /// The implementations of used abstract methods, by the class that is not used yet.
    waiting: FxHashMap<DefinitionId, Vec<DefinitionId>>,
    /// What is used so far, and the code that is left to follow. A code is left to follow once: a
    /// module at the start, and a function when it is first used.
    used: Vec<bool>,
    pending: Vec<CodeId>,
    seen: Seen,
}

/// What the uses of the code followed so far read, so that each is looked up once.
#[derive(Default)]
struct Seen {
    names: FxHashSet<(usize, Name)>,
    attributes: FxHashSet<Name>,
    keys: FxHashSet<String>,
    objects: FxHashSet<(usize, Vec<Name>)>,
    stars: FxHashSet<usize>,
    patterns: FxHashSet<(Vec<String>, Name)>,
    escaped: FxHashSet<usize>,
    /// The names of the modules read as objects, which any read of an attribute can stand for.
    loose: FxHashMap<Name, Vec<DefinitionId>>,
}

impl<'a> Graph<'a> {
    pub(super) fn new(project: &'a Project, uses: &'a AllUses, classes: &Classes) -> Self {
        let mut by_name: FxHashMap<(usize, Name), Vec<DefinitionId>> = FxHashMap::default();
        let mut by_attribute: FxHashMap<String, Vec<DefinitionId>> = FxHashMap::default();
        let mut by_key: FxHashMap<String, Vec<DefinitionId>> = FxHashMap::default();
        let mut public: FxHashMap<usize, Vec<DefinitionId>> = FxHashMap::default();
        for (id, file, definition) in project.definitions_by_file() {
            // Only a definition of the module itself is read by its name. A function or class that is a
            // local variable of a function, and a member of a class read in its class body, are read
            // by their definition.
            let is_local = project.facts[file]
                .references
                .local_definitions
                .contains(&(id - project.first_definition[file]));
            if !is_local && definition.class.is_none() {
                by_name
                    .entry((file, Name::new(&definition.name)))
                    .or_default()
                    .push(id);
            }
            // A field of a TypedDict is read as a key, and only a method or another attribute is read
            // as an attribute of an object.
            if classes.is_field(project, id) {
                by_key.entry(definition.name.clone()).or_default().push(id);
            } else if definition.class.is_some() {
                by_attribute
                    .entry(definition.name.clone())
                    .or_default()
                    .push(id);
            }
            // A star import imports the names in `__all__`, which are references of the module
            // already, or every public name of the module if there is no `__all__`. A method, an
            // attribute, or a function or class that is a local variable of a function is not a name
            // of the module.
            if !project.facts[file].references.has_all
                && !definition.name.starts_with('_')
                && definition.class.is_none()
                && !is_local
            {
                public.entry(file).or_default().push(id);
            }
        }
        let enums = classes
            .enums()
            .map(|(class, members)| {
                let names = members
                    .iter()
                    .map(|id| project.definitions[*id].name.clone())
                    .collect();
                let key = (
                    project.definition_files[class],
                    project.definitions[class].qualified_name.clone(),
                );
                (key, (members.to_vec(), names))
            })
            .collect();

        let mut module_names: FxHashMap<usize, Vec<(Name, DefinitionId)>> = FxHashMap::default();
        let mut module_level: FxHashMap<Name, Vec<DefinitionId>> = FxHashMap::default();
        for ((file, name), ids) in &by_name {
            let names = module_names.entry(*file).or_default();
            names.extend(ids.iter().map(|&id| (name.clone(), id)));
            module_level.entry(name.clone()).or_default().extend(ids);
        }

        Self {
            project,
            uses,
            enums,
            by_name,
            module_names,
            module_level,
            by_attribute,
            by_key,
            public,
            implementations: classes.implementations(project),
            waiting: FxHashMap::default(),
            used: vec![false; project.definitions.len()],
            // The code of every module runs.
            pending: (0..project.files.len()).collect(),
            seen: Seen::default(),
        }
    }

    /// Use more definitions, and get all the definitions that are used so far: the roots of every call,
    /// and the definitions that the code of the modules uses, directly or through the functions that
    /// it uses.
    ///
    /// The first call also uses what the code of the modules uses. Each later call goes on from where
    /// the one before stopped.
    pub(super) fn use_definitions(
        &mut self,
        roots: impl IntoIterator<Item = DefinitionId>,
    ) -> Vec<bool> {
        self.add(roots);
        while let Some(code) = self.pending.pop() {
            let Some(uses) = self.uses.get(code) else {
                continue;
            };
            let mut seen = std::mem::take(&mut self.seen);
            let mut found: Vec<DefinitionId> = Vec::new();
            self.look_up(uses, Some(&mut seen), &mut found);
            self.seen = seen;
            self.add(found);
        }
        self.used.clone()
    }

    /// Find the definitions that the code of a function uses directly, as if it were used. A definition
    /// that is not a function reaches nothing.
    ///
    /// An abstract method also reaches its implementations, whether their class is used or not: an
    /// implementation in an unused class is reached through its class anyway.
    pub(super) fn reached(&self, id: DefinitionId) -> FxHashSet<DefinitionId> {
        let mut reached = FxHashSet::default();
        if !self.project.definitions[id].kind.is_function() {
            return reached;
        }
        let implementations = self.implementations.get(&id).into_iter().flatten();
        reached.extend(implementations.map(|&(_, method)| method));
        let Some(uses) = self.uses.get(self.project.function_code(id)) else {
            return reached;
        };
        self.look_up(uses, None, &mut reached);
        reached
    }

    /// Find the definitions that the uses of a code can stand for: the names by their module, the
    /// attributes and keys by their name, the members of the enums that are read as a whole, and the
    /// public names of the modules that are star imported, and the definitions that the code reads
    /// exactly: functions and classes defined in a function, and members found through a class.
    ///
    /// With `seen`, a use that is in it already is skipped, and the others are added to it.
    fn look_up(
        &self,
        uses: &Uses,
        mut seen: Option<&mut Seen>,
        found: &mut impl Extend<DefinitionId>,
    ) {
        for name in &uses.names {
            if seen
                .as_mut()
                .is_none_or(|seen| seen.names.insert(name.clone()))
            {
                found.extend(self.by_name.get(name).into_iter().flatten().copied());
            }
        }
        for attribute in &uses.attributes {
            if seen
                .as_mut()
                .is_none_or(|seen| seen.attributes.insert(attribute.clone()))
            {
                found.extend(
                    self.by_attribute
                        .get(attribute.as_str())
                        .into_iter()
                        .flatten()
                        .copied(),
                );
                let loose = &seen.as_deref().unwrap_or(&self.seen).loose;
                found.extend(loose.get(attribute).into_iter().flatten().copied());
            }
        }
        for read in &uses.patterns {
            if seen
                .as_mut()
                .is_none_or(|seen| seen.patterns.insert(read.clone()))
            {
                found.extend(self.pattern_read(&read.0, &read.1));
            }
        }
        for &module in &uses.escaped {
            let names = self.module_names.get(&module).into_iter().flatten();
            match seen.as_mut() {
                Some(seen) => {
                    if seen.escaped.insert(module) {
                        for (name, id) in names {
                            if seen.attributes.contains(name) {
                                found.extend([*id]);
                            }
                            seen.loose.entry(name.clone()).or_default().push(*id);
                        }
                    }
                }
                None => found.extend(
                    names
                        .filter(|(name, _)| self.seen.attributes.contains(name))
                        .map(|(_, id)| *id),
                ),
            }
        }
        for key in &uses.keys {
            if seen
                .as_mut()
                .is_none_or(|seen| seen.keys.insert(key.clone()))
            {
                found.extend(self.by_key.get(key).into_iter().flatten().copied());
            }
        }
        for read in &uses.objects {
            if seen
                .as_mut()
                .is_none_or(|seen| seen.objects.insert(read.clone()))
            {
                found.extend(self.enums_read(read.0, &read.1));
            }
        }
        for star in &uses.stars {
            if seen.as_mut().is_none_or(|seen| seen.stars.insert(*star)) {
                found.extend(self.public.get(star).into_iter().flatten().copied());
            }
        }
        found.extend(uses.definitions.iter().copied());
    }

    /// Find the definitions of a name in the modules whose names match a pattern, given by its literal
    /// parts as `ModulePattern` holds them.
    fn pattern_read(&self, parts: &[String], name: &Name) -> impl Iterator<Item = DefinitionId> {
        let ids = self.module_level.get(name).into_iter().flatten();
        ids.copied().filter(move |&id| {
            self.project
                .matches_pattern(self.project.definition_files[id], parts)
        })
    }

    /// Find the members of the enums that an object read uses as a whole.
    ///
    /// A member read as an attribute, as in `Color.RED`, is used by its name. Any other read of an enum,
    /// such as `Color(value)`, `list(Color)`, `Color.parse(text)` or an annotation, can reach any
    /// member, so it uses them all.
    fn enums_read(&self, file: usize, parts: &[Name]) -> Vec<DefinitionId> {
        let mut found = Vec::new();
        for index in 1..=parts.len() {
            if let Some((members, names)) = self.enums.get(&(file, parts[..index].join(".")))
                && (index == parts.len() || !names.contains(parts[index].as_str()))
            {
                found.extend(members);
            }
        }
        found
    }

    /// Mark definitions as used, with the implementations of abstract methods that they make used, and
    /// queue the code of the functions among them.
    fn add(&mut self, definitions: impl IntoIterator<Item = DefinitionId>) {
        let mut queue: Vec<DefinitionId> = definitions.into_iter().collect();
        while let Some(id) = queue.pop() {
            if self.used[id] {
                continue;
            }
            self.used[id] = true;
            if self.project.definitions[id].kind.is_function() {
                self.pending.push(self.project.function_code(id));
            }
            for &(class, method) in self.implementations.get(&id).into_iter().flatten() {
                if self.used[class] {
                    queue.push(method);
                } else {
                    self.waiting.entry(class).or_default().push(method);
                }
            }
            queue.extend(self.waiting.remove(&id).into_iter().flatten());
        }
    }
}
