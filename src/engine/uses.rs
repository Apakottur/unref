//! What the code of each owner of a project uses: the names, attributes, keys and objects it reads.

use ruff_python_ast::name::Name;
use rustc_hash::FxHashSet;

use crate::references::Receiver;
use crate::resolver::{Resolver, imported_name, join};

use super::{Classes, CodeId, DefinitionId, Project, is_external, is_external_import};

/// What the code of an owner uses.
#[derive(Default)]
pub(super) struct Uses {
    /// The names of the modules of the project that the code uses, each with its module.
    pub(super) names: FxHashSet<(usize, Name)>,
    /// The names that the code reads as attributes of an object.
    pub(super) attributes: FxHashSet<Name>,
    /// The string keys that the code reads from a mapping.
    pub(super) keys: FxHashSet<String>,
    /// The objects of the modules of the project that the code reads, each with its module and its
    /// name in the module followed by the attributes that are read from it, in order.
    pub(super) objects: FxHashSet<(usize, Vec<Name>)>,
    /// The names that the code reads from some module whose name matches a pattern, each with the
    /// literal parts of the pattern, the first resolved, as `ModulePattern` holds them.
    pub(super) patterns: FxHashSet<(Vec<String>, Name)>,
    /// The modules of the project that the code reads as objects, other than to read a name from them,
    /// such as by passing them to a function. Any name read from an object can be read from them.
    pub(super) escaped: FxHashSet<usize>,
    /// The modules that the code star imports.
    pub(super) stars: FxHashSet<usize>,
    /// The functions and classes that are local variables of a function, the members of a class read
    /// in its class body, and the members that a read through a class or its first parameter can
    /// stand for, that the code reads.
    pub(super) definitions: FxHashSet<DefinitionId>,
}

/// What the code of each module and function of a project uses, by its code. Most functions use
/// nothing that is kept here, so the uses are packed and each code has the position of its own.
pub(super) struct AllUses {
    /// The position of the uses of each code, or `NONE` if it uses nothing.
    slots: Vec<u32>,
    uses: Vec<Uses>,
}

impl AllUses {
    const NONE: u32 = u32::MAX;

    fn new(codes: usize) -> Self {
        Self {
            slots: vec![Self::NONE; codes],
            uses: Vec::new(),
        }
    }

    /// Get the uses of a code, to add to them.
    fn entry(&mut self, code: CodeId) -> &mut Uses {
        let slot = &mut self.slots[code];
        if *slot == Self::NONE {
            *slot = u32::try_from(self.uses.len()).expect("fewer codes than u32::MAX");
            self.uses.push(Uses::default());
        }
        &mut self.uses[*slot as usize]
    }

    /// Get the uses of a code, or None if it uses nothing.
    pub(super) fn get(&self, code: CodeId) -> Option<&Uses> {
        let slot = self.slots[code];
        (slot != Self::NONE).then(|| &self.uses[slot as usize])
    }
}

/// Find what the code of each owner uses.
///
/// A plain name refers to the module that it is read in, and an imported name to the module that it
/// is imported from. A member of a class of the project that is plain, as in `self.name`,
/// `super().name`, `Class.name` or `Class().name`, is matched in the classes that it can be in, and
/// any other attribute of an object by name only. A TypedDict of the project that is read as a whole,
/// or passed to a function from outside the project, has all its fields used, and those of the
/// TypedDicts that it inherits from.
pub(super) fn find_uses(project: &Project, classes: &Classes) -> AllUses {
    let mut uses = AllUses::new(project.files.len() + project.definitions.len());
    let resolver = classes.hierarchy.resolver();
    // The names of the TypedDicts, so that a value that cannot be one is not resolved.
    let typed_dict_names: FxHashSet<&str> = classes
        .typed_dicts()
        .map(|(class, _)| project.definitions[class].name.as_str())
        .collect();
    for (file, file_facts) in project.facts.iter().enumerate() {
        let references = &file_facts.references;
        let module = &project.names[file].name;
        for read in &references.reads {
            let owner_uses = uses.entry(project.code(file, read.owner));
            owner_uses.names.insert((file, read.root.clone()));
            let mut parts = vec![read.root.clone()];
            parts.extend(read.attributes.iter().cloned());
            // Each attribute, and the one read from the result of the call, is read from the chain
            // before it.
            for (index, attribute) in read.attributes.iter().chain(&read.called).enumerate() {
                owner_uses.add_member_of(classes, module, &parts[..=index], attribute);
            }
            if read.reads_object {
                owner_uses.objects.insert((file, parts));
            }
        }

        for read in &references.imported_reads {
            let owner_uses = uses.entry(project.code(file, read.owner));
            let Some(imported) = imported_name(&project.names[file], &read.binding) else {
                owner_uses
                    .attributes
                    .extend(read.attributes.iter().chain(&read.called).cloned());
                continue;
            };
            let mut parts: Vec<Name> = imported.split('.').map(Name::new).collect();
            parts.extend(read.attributes.iter().cloned());
            // A module of the project read as an object, other than to read a name from it.
            if read.passed {
                owner_uses
                    .escaped
                    .extend(passed_modules(project, &parts).iter().copied());
            }
            if read.reads_object || !read.attributes.is_empty() {
                let objects = imported_objects(project, resolver, &parts.join("."));
                if !read.attributes.is_empty() {
                    // An imported module can be reached through a re-export. Its attributes
                    // are names of the original module, just as they are for a direct import.
                    owner_uses.names.extend(
                        objects
                            .iter()
                            .map(|(file, parts)| (*file, parts[0].clone())),
                    );
                }
                if read.reads_object {
                    owner_uses.objects.extend(objects);
                }
            }
            if let Some(called) = &read.called {
                owner_uses.add_member_of(classes, "", &parts, called);
            }
            if read.attributes.is_empty() {
                continue;
            }
            // What follows the last name that a module of the project holds is read from an object.
            let first_of_object = (1..parts.len())
                .filter(|&index| {
                    project
                        .files_by_name
                        .contains_key(&parts[..index].join("."))
                })
                .map(|index| index + 1)
                .chain([parts.len() - read.attributes.len()])
                .max()
                .unwrap_or(0);
            for index in first_of_object..parts.len() {
                owner_uses.add_member_of(classes, "", &parts[..index], &parts[index]);
            }
        }

        for import in &references.imports {
            let module = project.names[file]
                .resolve(import.module.as_deref(), import.level)
                .unwrap_or_default();
            for &imported in project.files_by_name.get(&module).into_iter().flatten() {
                let owner_uses = uses.entry(project.code(file, import.owner));
                if import.name == "*" {
                    owner_uses.stars.insert(imported);
                } else {
                    owner_uses.names.insert((imported, Name::new(&import.name)));
                }
            }
        }

        for (owner, pattern) in &references.passed_patterns {
            let mut parts = pattern.parts.clone();
            if let Some(first) = project.names[file].resolve(Some(&parts[0]), pattern.level) {
                parts[0] = first;
                let matching = (0..project.files.len())
                    .filter(|&module| project.matches_pattern(module, &parts));
                uses.entry(project.code(file, *owner))
                    .escaped
                    .extend(matching);
            }
        }
        for (owner, pattern, name) in &references.pattern_reads {
            let mut parts = pattern.parts.clone();
            if let Some(first) = project.names[file].resolve(Some(&parts[0]), pattern.level) {
                parts[0] = first;
                uses.entry(project.code(file, *owner))
                    .patterns
                    .insert((parts, name.clone()));
            }
        }
        for (owner, name) in &references.attributes {
            uses.entry(project.code(file, *owner))
                .attributes
                .insert(name.clone());
        }
        for (owner, receiver, name) in &references.members {
            let (Receiver::Own(class) | Receiver::Super(class)) = *receiver;
            let is_super = matches!(receiver, Receiver::Super(_));
            let found = classes.read_from_own(project.id(file, class), name, is_super);
            uses.entry(project.code(file, *owner))
                .add_member(found, name);
        }
        for (owner, name) in &references.keys {
            uses.entry(project.code(file, *owner))
                .keys
                .insert(name.clone());
        }
        for read in &references.whole_reads {
            // A TypedDict is called by its name, or by a name that it is imported as.
            let (root, _) = read.value.split_once('.').unwrap_or((&read.value, ""));
            let last = read.value.rsplit('.').next().unwrap_or(&read.value);
            let imported = references
                .bindings
                .get(root)
                .and_then(|binding| binding.name.as_deref());
            if !typed_dict_names.contains(last)
                && !imported.is_some_and(|name| typed_dict_names.contains(name))
            {
                continue;
            }
            let fields: Vec<DefinitionId> = classes
                .hierarchy
                .resolve(file, &read.value)
                .iter()
                .flat_map(|dotted| classes.fields_of(dotted))
                .collect();
            if !fields.is_empty()
                && read
                    .reader
                    .as_ref()
                    .is_none_or(|reader| match &read.local_import {
                        Some(binding) => is_external_import(project, file, binding, reader),
                        None => is_external(project, resolver, file, reader),
                    })
            {
                uses.entry(project.code(file, read.owner))
                    .definitions
                    .extend(fields);
            }
        }
        for (owner, index) in &references.local_reads {
            uses.entry(project.code(file, *owner))
                .definitions
                .insert(project.first_definition[file] + index);
        }
        // The namespace of the module read as a whole reads each of its names.
        for owner in &references.namespace_reads {
            let names = project
                .file_definitions(file)
                .iter()
                .filter(|definition| definition.class.is_none())
                .map(|definition| (file, Name::new(&definition.name)));
            uses.entry(project.code(file, *owner)).names.extend(names);
        }
        // An object of a function is read by its name in the module, such as an enum in a function.
        for (owner, index, attributes) in &references.local_objects {
            let definition = &project.definitions[project.first_definition[file] + index];
            let mut parts = vec![Name::new(&definition.qualified_name)];
            parts.extend(attributes.iter().cloned());
            uses.entry(project.code(file, *owner))
                .objects
                .insert((file, parts));
        }
    }
    uses
}

/// Find the modules of the project that a dotted name, as imported, stands for. A name of a package
/// stands for its module unless the package binds the name to something else, as
/// `from .directive import directive` does.
fn passed_modules<'a>(project: &'a Project, parts: &[Name]) -> &'a [usize] {
    let full = parts.join(".");
    let Some(files) = project.files_by_name.get(&full) else {
        return &[];
    };
    let Some((last, package)) = parts.split_last() else {
        return &[];
    };
    let package = package.join(".");
    for &file in project.files_by_name.get(&package).into_iter().flatten() {
        let references = &project.facts[file].references;
        let rebound = references
            .bindings
            .get(last.as_str())
            .is_some_and(|binding| {
                imported_name(&project.names[file], binding).is_none_or(|name| name != full)
            });
        let defined = project
            .file_definitions(file)
            .iter()
            .any(|definition| definition.class.is_none() && definition.name == last.as_str());
        if rebound || defined {
            return &[];
        }
    }
    files
}

/// Find the objects of the modules of the project that a dotted name stands for, through the modules
/// that import it in turn, star import it or give it out from their `__getattr__`.
///
/// Returns each object with each module of the project that a start of one of the names of it is, and
/// the rest of that name.
fn imported_objects(
    project: &Project,
    resolver: &Resolver,
    name: &str,
) -> FxHashSet<(usize, Vec<Name>)> {
    let mut found = FxHashSet::default();
    for alias in resolver.project_aliases(name).iter() {
        for (index, _) in alias.match_indices('.') {
            for &module in project
                .files_by_name
                .get(&alias[..index])
                .into_iter()
                .flatten()
            {
                let rest = alias[index + 1..].split('.').map(Name::new).collect();
                found.insert((module, rest));
            }
        }
    }
    found
}

impl Uses {
    /// Add a read of a member from a chain of names, such as `name` from `package.module.Class`.
    ///
    /// `module` is the module that the chain is written in, or empty for a chain that starts with
    /// the name of a module.
    fn add_member_of(&mut self, classes: &Classes, module: &str, chain: &[Name], name: &Name) {
        let written = join(module, &chain.join("."));
        self.add_member(classes.read_from(&written, name), name);
    }

    /// Add a read of a member: the definitions that it can stand for, or its name if it is not known
    /// what it is read from.
    fn add_member(&mut self, found: Option<Vec<DefinitionId>>, name: &Name) {
        match found {
            Some(found) => self.definitions.extend(found),
            None => {
                self.attributes.insert(name.clone());
            }
        }
    }
}
