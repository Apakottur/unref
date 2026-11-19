//! The unused definitions of a Python project, which are those that nothing references.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use regex::Regex;
use ruff_python_ast::name::Name;
use ruff_python_stdlib::builtins::is_python_builtin;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::classes::{ClassHierarchy, join};
use crate::config::Config;
use crate::definitions::{Definition, Kind};
use crate::discover::{discover, is_under};
use crate::external::{ExternalModules, environment_paths, standard_library};
use crate::files::{FileFacts, analyze_files};
use crate::frameworks::{FrameworkRules, ProjectFile};
use crate::ignores::{apply_ignore_comments, ignored_codes};
use crate::models::{Finding, Framework, Override, fnmatch};
use crate::modules::{ModuleName, import_root, module_name, resolve};
use crate::references::{Binding, Owner, Receiver};
use crate::symbols::ModuleSymbols;
use crate::transitive::transitive;

/// The object reference of an entry point, as the entry points specification of the Python Packaging
/// User Guide gives it.
static OBJECT_REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<module>[\w.]+)\s*(:\s*(?P<attributes>[\w.]+)\s*)?(\[.*\]\s*)?$")
        .expect("valid pattern")
});
/// `TypedDict`, as `typing` and `typing_extensions` give it out.
const TYPED_DICTS: [&str; 2] = ["typing.TypedDict", "typing_extensions.TypedDict"];
/// `Enum`, which `IntEnum`, `StrEnum`, `Flag` and the other enums of `enum` inherit from.
const ENUMS: [&str; 1] = ["enum.Enum"];
/// `ast.NodeVisitor`, whose `visit` calls the method named `visit_` and the class of the node, and
/// `ast.NodeTransformer`, which inherits from it.
const NODE_VISITORS: [&str; 2] = ["ast.NodeVisitor", "ast.NodeTransformer"];
const NODE_VISITOR_PREFIX: &str = "visit_";
/// `Protocol`, as `typing` and `typing_extensions` give it out, whose members a class has without
/// inheriting from it.
const PROTOCOLS: [&str; 2] = ["typing.Protocol", "typing_extensions.Protocol"];
const ABSTRACT_METHOD: &str = "abc.abstractmethod";

/// A definition of the project, by its index among the definitions of every file in order.
type DefinitionId = usize;
/// A class of the project, by its index among the classes.
type ClassId = usize;

/// What the code of an owner uses.
#[derive(Default)]
struct Uses {
    /// The names of the modules of the project that the code uses, each with its module.
    names: FxHashSet<(usize, Name)>,
    /// The names that the code reads as attributes of an object.
    attributes: FxHashSet<Name>,
    /// The string keys that the code reads from a mapping.
    keys: FxHashSet<String>,
    /// The objects of the modules of the project that the code reads, each with its module and its
    /// name in the module followed by the attributes that are read from it, in order.
    objects: FxHashSet<(usize, Vec<Name>)>,
    /// The modules that the code star imports.
    stars: FxHashSet<usize>,
    /// The functions and classes that are local variables of a function, the members of a class read
    /// in its class body, and the members that a read through a class or its first parameter can
    /// stand for, that the code reads.
    definitions: FxHashSet<DefinitionId>,
}

type UsesByOwner = FxHashMap<(usize, Owner), Uses>;

#[derive(Default)]
struct ProjectUses {
    ordinary: UsesByOwner,
    sibling: UsesByOwner,
}

/// Imports that can take advantage of a script's search directory.
#[derive(Default)]
struct SiblingImports {
    roots: Vec<usize>,
    qualifications: FxHashMap<usize, FxHashSet<String>>,
    loads: FxHashMap<(usize, Owner), FxHashSet<usize>>,
}

impl SiblingImports {
    fn qualifies(&self, file: usize, module: Option<&str>, level: u32) -> bool {
        level == 0
            && module.is_some_and(|module| {
                self.qualifications
                    .get(&file)
                    .is_some_and(|names| names.contains(module))
            })
    }
}

/// The modules of the project, by the index of their file.
struct Project {
    /// The files that parse.
    files: Vec<Arc<Path>>,
    /// What each file defines and refers to.
    facts: Vec<FileFacts>,
    /// The module name of each file.
    names: Vec<ModuleName>,
    /// The files of each module name.
    files_by_name: FxHashMap<String, Vec<usize>>,
    /// The definitions of every file, in order.
    definitions: Vec<Definition>,
    /// The file of each definition.
    definition_files: Vec<usize>,
    /// The first definition of each file.
    first_definition: Vec<DefinitionId>,
}

/// Find the unused definitions of a Python project, made up of the given files and directories, which
/// are those that nothing references.
///
/// Returns a finding for each unused definition, for each problem with an ignore comment, and for each
/// file that does not parse or path that cannot be read, sorted by path, line and column.
pub(crate) fn find_unused(options: &Config, paths: &[PathBuf]) -> Vec<Finding> {
    let enabled: FxHashSet<Framework> = Framework::ALL
        .into_iter()
        .filter(|framework| options.frameworks.get(framework).copied().unwrap_or(true))
        .collect();

    let discovery = discover(paths, options);
    let discovered: Vec<Arc<Path>> = discovery.files.into_iter().map(Arc::from).collect();
    let mut syntax_errors: FxHashMap<Arc<Path>, Finding> = FxHashMap::default();
    let mut files = Vec::new();
    let mut facts = Vec::new();
    for (path, result) in discovered.iter().zip(analyze_files(&discovered)) {
        match result {
            Ok(file_facts) => {
                files.push(path.clone());
                facts.push(file_facts);
            }
            Err(finding) => {
                syntax_errors.insert(path.clone(), finding);
            }
        }
    }

    // Only a directory can have a namespace package under it. A hook of prek or pre-commit gives every
    // file of the project, and checking each directory against each of them takes quadratic time.
    let roots: Vec<PathBuf> = paths
        .iter()
        .map(|path| resolve(path))
        .filter(|root| root.is_dir())
        .collect();
    let names: Vec<ModuleName> = files.iter().map(|path| module_name(path, &roots)).collect();
    let mut files_by_name: FxHashMap<String, Vec<usize>> = FxHashMap::default();
    for (index, name) in names.iter().enumerate() {
        files_by_name
            .entry(name.name.clone())
            .or_default()
            .push(index);
    }
    let mut external = ExternalModules::new(search_path(&files, &names, options.python.as_deref()));
    let siblings = import_siblings(&mut facts, &names, &files_by_name, &mut external);

    let mut definitions = Vec::new();
    let mut definition_files = Vec::new();
    let mut first_definition = Vec::new();
    for (index, file_facts) in facts.iter_mut().enumerate() {
        first_definition.push(definitions.len());
        let file_definitions = std::mem::take(&mut file_facts.definitions);
        definition_files.extend(std::iter::repeat_n(index, file_definitions.len()));
        definitions.extend(file_definitions);
    }
    let project = Project {
        files,
        facts,
        names,
        files_by_name,
        definitions,
        definition_files,
        first_definition,
    };

    let modules: Vec<Arc<ModuleSymbols>> = (0..project.files.len())
        .map(|index| {
            let file_facts = &project.facts[index];
            let mut symbols = ModuleSymbols::of(
                project.files[index].clone(),
                project.names[index].clone(),
                project.file_definitions(index),
                &file_facts.references,
                file_facts.lazy.clone(),
            );
            for star in &mut symbols.stars {
                if siblings.qualifies(index, Some(star), 0)
                    && let Some(qualified) = project.names[index].resolve(Some(star), 1)
                {
                    *star = qualified;
                }
            }
            symbols.stars.sort();
            symbols.stars.dedup();
            Arc::new(symbols)
        })
        .collect();
    let hierarchy = ClassHierarchy::new(modules, project.files_by_name.clone(), external);
    let typed_dicts = subclass_attributes(&project, &hierarchy, &TYPED_DICTS);
    let enums = subclass_attributes(&project, &hierarchy, &ENUMS);
    let fields: FxHashSet<DefinitionId> = typed_dicts.values().flatten().copied().collect();
    let classes = Classes::new(&project, &hierarchy, &fields);
    let typed_dicts: FxHashMap<String, Vec<DefinitionId>> = typed_dicts
        .into_iter()
        .map(|((file, class), fields)| (join(&project.names[file].name, &class), fields))
        .collect();
    let uses = find_uses(&project, &classes, &typed_dicts, &siblings);

    let file_definitions: Vec<ProjectFile> = (0..project.files.len())
        .map(|index| {
            (
                &*project.files[index],
                project.file_definitions(index),
                &project.facts[index].references,
            )
        })
        .collect();
    let sphinx_configuration: Option<FxHashSet<PathBuf>> = options
        .sphinx_configuration
        .as_ref()
        .map(|files| files.iter().map(|file| resolve(file)).collect());
    let django_settings: FxHashSet<PathBuf> = options
        .django_settings
        .iter()
        .map(|file| resolve(file))
        .collect();
    let rules = FrameworkRules::new(
        &enabled,
        &file_definitions,
        &project.names,
        &hierarchy,
        sphinx_configuration.as_ref(),
        &django_settings,
    );
    let (kept, ignored) = find_roots(&project, &hierarchy, options, &rules);

    let implementations = classes.implementations(&project);
    let mut graph = Graph::new(&project, &uses, &siblings, &fields, enums, implementations);

    // An ignore comment is needed for a definition that is unused without it. Such a definition is
    // then used, and so is what it uses.
    let kept_used = graph.use_definitions(kept.iter().copied());
    let needed: Vec<DefinitionId> = ignored.into_iter().filter(|id| !kept_used[*id]).collect();
    let all_used = graph.use_definitions(needed.iter().copied());
    let unused: Vec<DefinitionId> = (0..project.definitions.len())
        .filter(|id| !all_used[*id])
        .collect();
    let is_transitive = transitive_definitions(&project, &graph, &unused);

    let mut findings = findings(
        &project,
        &discovered,
        &syntax_errors,
        &all_used,
        &needed,
        &is_transitive,
    );
    if !discovery.errors.is_empty() {
        findings.extend(discovery.errors);
        findings.sort_by(|left, right| {
            (&left.path, left.line, left.column).cmp(&(&right.path, right.line, right.column))
        });
    }

    // The command exits after the check, and the system frees its memory at once, which is faster than
    // dropping the many small parts of these one by one, as clang does too. A caller that checks many
    // times in one process would need them dropped.
    std::mem::forget(graph);
    std::mem::forget(rules);
    std::mem::forget(file_definitions);
    std::mem::forget(uses);
    std::mem::forget(siblings);
    std::mem::forget(classes);
    std::mem::forget(hierarchy);
    std::mem::forget(project);

    findings
}

impl Project {
    /// The definitions of a file, by their ids.
    fn ids_of_file(&self, file: usize) -> std::ops::Range<DefinitionId> {
        let start = self.first_definition[file];
        let end = self
            .first_definition
            .get(file + 1)
            .copied()
            .unwrap_or(self.definitions.len());
        start..end
    }

    /// The definitions of a file.
    fn file_definitions(&self, file: usize) -> &[Definition] {
        &self.definitions[self.ids_of_file(file)]
    }

    /// Every definition, with its id and its file.
    fn definitions_by_file(&self) -> impl Iterator<Item = (DefinitionId, usize, &Definition)> {
        self.definition_files
            .iter()
            .zip(&self.definitions)
            .enumerate()
            .map(|(id, (&file, definition))| (id, file, definition))
    }

    /// Check whether a dotted name is in a module of the project: whether a start of it, shorter than
    /// the whole, is the name of one.
    fn holds(&self, name: &str) -> bool {
        name.match_indices('.')
            .any(|(index, _)| self.files_by_name.contains_key(&name[..index]))
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
///
/// `typed_dicts` are the fields of each TypedDict of the project, by its dotted name.
fn find_uses(
    project: &Project,
    classes: &Classes,
    typed_dicts: &FxHashMap<String, Vec<DefinitionId>>,
    siblings: &SiblingImports,
) -> ProjectUses {
    let mut all_uses = ProjectUses::default();
    let uses = &mut all_uses.ordinary;
    // The names of the TypedDicts, so that a value that cannot be one is not resolved.
    let typed_dict_names: FxHashSet<&str> = typed_dicts
        .keys()
        .map(|dotted| dotted.rsplit('.').next().unwrap_or(dotted))
        .collect();
    for (file, file_facts) in project.facts.iter().enumerate() {
        let references = &file_facts.references;
        let module = &project.names[file].name;
        for read in &references.reads {
            let owner_uses = uses.entry((file, read.owner)).or_default();
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
            let is_sibling =
                siblings.qualifies(file, read.binding.module.as_deref(), read.binding.level);
            let uses = if is_sibling {
                &mut all_uses.sibling
            } else {
                &mut *uses
            };
            let owner_uses = uses.entry((file, read.owner)).or_default();
            let binding;
            let binding = if is_sibling {
                binding = Binding {
                    level: 1,
                    ..read.binding.clone()
                };
                &binding
            } else {
                &read.binding
            };
            let Some(resolved) = resolve_import(
                &project.names[file],
                binding,
                &read.attributes,
                &project.files_by_name,
            ) else {
                owner_uses
                    .attributes
                    .extend(read.attributes.iter().chain(&read.called).cloned());
                continue;
            };
            if read.reads_object || !read.attributes.is_empty() {
                let objects = imported_objects(&resolved, project);
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
                owner_uses.add_member_of(classes, "", &resolved.0, called);
            }
            if read.attributes.is_empty() {
                continue;
            }
            let (parts, starts) = &resolved;
            // What follows the last name that a module of the project holds is read from an object.
            let first_of_object = starts
                .iter()
                .map(|(index, _)| index + 1)
                .chain([parts.len() - read.attributes.len()])
                .max()
                .unwrap_or(0);
            for index in first_of_object..parts.len() {
                owner_uses.add_member_of(classes, "", &parts[..index], &parts[index]);
            }
        }

        for import in &references.imports {
            let is_sibling = siblings.qualifies(file, import.module.as_deref(), import.level);
            let uses = if is_sibling {
                &mut all_uses.sibling
            } else {
                &mut *uses
            };
            let module = project.names[file]
                .resolve(
                    import.module.as_deref(),
                    if is_sibling { 1 } else { import.level },
                )
                .unwrap_or_default();
            for &imported in project.files_by_name.get(&module).into_iter().flatten() {
                let owner_uses = uses.entry((file, import.owner)).or_default();
                if import.name == "*" {
                    owner_uses.stars.insert(imported);
                } else {
                    owner_uses.names.insert((imported, Name::new(&import.name)));
                }
            }
        }

        for (owner, name) in &references.attributes {
            uses.entry((file, *owner))
                .or_default()
                .attributes
                .insert(name.clone());
        }
        for (owner, receiver, name) in &references.members {
            let (Receiver::Own(class) | Receiver::Super(class)) = receiver;
            let is_super = matches!(receiver, Receiver::Super(_));
            let found = classes.read_from_own(file, class, name, is_super);
            uses.entry((file, *owner))
                .or_default()
                .add_member(found, name);
        }
        for (owner, name) in &references.keys {
            uses.entry((file, *owner))
                .or_default()
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
                .flat_map(|dotted| typed_dicts.get(dotted).into_iter().flatten().copied())
                .collect();
            if !fields.is_empty()
                && read
                    .reader
                    .as_ref()
                    .is_none_or(|reader| is_external(project, classes.hierarchy, file, reader))
            {
                uses.entry((file, read.owner))
                    .or_default()
                    .definitions
                    .extend(fields);
            }
        }
        for (owner, index) in &references.local_reads {
            uses.entry((file, *owner))
                .or_default()
                .definitions
                .insert(project.first_definition[file] + index);
        }
    }
    all_uses
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

/// The classes of the project, with their members, so that a member read from an object whose class
/// is plain is matched in the classes that the object can be an instance of.
struct Classes<'a> {
    hierarchy: &'a ClassHierarchy,
    /// Each class by its file and its name in the module.
    ids: FxHashMap<(usize, String), ClassId>,
    /// The definition of each class.
    definitions: Vec<DefinitionId>,
    /// The classes by their dotted names, such as `package.module.Class`.
    by_dotted: FxHashMap<String, Vec<ClassId>>,
    /// The names of the classes, without their modules, so that a name that cannot be a class is not
    /// followed.
    names: FxHashSet<String>,
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
    fn new(
        project: &Project,
        hierarchy: &'a ClassHierarchy,
        fields: &FxHashSet<DefinitionId>,
    ) -> Self {
        let mut keys: Vec<(usize, String)> = Vec::new();
        let mut ids = FxHashMap::default();
        let mut definitions = Vec::new();
        let mut by_dotted: FxHashMap<String, Vec<ClassId>> = FxHashMap::default();
        let mut names = FxHashSet::default();
        for (definition_id, (file, definition)) in project
            .definition_files
            .iter()
            .zip(&project.definitions)
            .enumerate()
        {
            if definition.kind == Kind::Class {
                let key = (*file, definition.qualified_name());
                if ids.contains_key(&key) {
                    continue;
                }
                let id = keys.len();
                ids.insert(key.clone(), id);
                definitions.push(definition_id);
                let dotted = join(&project.names[*file].name, &key.1);
                by_dotted.entry(dotted).or_default().push(id);
                names.insert(definition.name.clone());
                keys.push(key);
            }
        }
        let mut ancestors = Vec::new();
        let mut metaclasses = Vec::new();
        let mut protocols = vec![false; keys.len()];
        let mut unknown_bases = vec![false; keys.len()];
        for (file, definition) in project.definition_files.iter().zip(&project.definitions) {
            if definition.kind != Kind::Class {
                continue;
            }
            // The loop above gave an id to every class.
            let name = definition.qualified_name();
            let id = ids[&(*file, name.clone())];
            unknown_bases[id] |= definition.other_bases;
            for base in &definition.bases {
                let names = hierarchy.resolve_in(*file, &name, base);
                protocols[id] |= PROTOCOLS.iter().any(|protocol| names.contains(*protocol));
                unknown_bases[id] |= is_unknown_class(project, &by_dotted, *file, base, &names);
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
        for (id, (file, definition)) in project
            .definition_files
            .iter()
            .zip(&project.definitions)
            .enumerate()
        {
            if let Some(class_name) = &definition.class
                && !fields.contains(&id)
                && let Some(&class) = ids.get(&(*file, class_name.clone()))
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
            names,
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
    fn members(
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
    fn read_from(&self, written: &str, name: &str) -> Option<Vec<DefinitionId>> {
        let last = written.rsplit('.').next().unwrap_or(written);
        if !self.names.contains(last) {
            return None;
        }
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
    fn read_from_own(
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
    fn implementations(
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
                        .resolve(file, decorator)
                        .contains(ABSTRACT_METHOD)
                })
            {
                implementations.insert(id, found);
            }
        }
        implementations
    }
}

/// Check whether a function, as it is written in a module, is from outside the project: a builtin, or
/// a name that the module imports from outside the project. Any other name, such as a method of a local
/// variable, is not known.
fn is_external(project: &Project, hierarchy: &ClassHierarchy, file: usize, written: &str) -> bool {
    let root = written.split('.').next().unwrap_or(written);
    // The builtins of any version of Python.
    let is_known = project.facts[file].references.bindings.contains_key(root)
        || is_python_builtin(root, u8::MAX, false);
    is_known
        && hierarchy
            .resolve(file, written)
            .iter()
            .all(|name| !project.holds(name))
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

/// The dotted name that an imported name and the attributes read from it stand for, as its parts,
/// and each shorter start of it that is a module of the project, shortest first, as its number of
/// parts with its files.
type Resolved = (Vec<Name>, Vec<(usize, Vec<usize>)>);

/// Find the dotted name that an imported name, and the attributes read from it, stand for.
///
/// Returns None if it is imported from beyond the root.
fn resolve_import(
    module: &ModuleName,
    binding: &Binding,
    attributes: &[Name],
    files_by_name: &FxHashMap<String, Vec<usize>>,
) -> Option<Resolved> {
    let resolved = module.resolve(binding.module.as_deref(), binding.level)?;
    let mut parts: Vec<Name> = resolved.split('.').map(Name::new).collect();
    if let Some(name) = binding.name.as_ref().filter(|name| !name.is_empty()) {
        parts.push(Name::new(name));
    }
    parts.extend(attributes.iter().cloned());
    let starts = (1..parts.len())
        .filter_map(|index| {
            files_by_name
                .get(&parts[..index].join("."))
                .map(|files| (index, files.clone()))
        })
        .collect();
    Some((parts, starts))
}

/// Find the objects of the modules of the project that an imported name, and the attributes read from
/// it, stand for, through the modules that import the name in turn.
///
/// Returns each object with its module, and its name in the module followed by the attributes.
fn imported_objects(resolved: &Resolved, project: &Project) -> FxHashSet<(usize, Vec<Name>)> {
    let mut found = FxHashSet::default();
    // Each binding that is followed is followed once, which ends a cycle of modules that import the
    // names of each other.
    let mut followed: FxHashSet<(usize, Name)> = FxHashSet::default();
    let mut pending = vec![resolved.clone()];
    while let Some((parts, starts)) = pending.pop() {
        let Some((index, modules)) = starts.last() else {
            continue;
        };
        for (start, start_modules) in &starts {
            found.extend(
                start_modules
                    .iter()
                    .map(|&module| (module, parts[*start..].to_vec())),
            );
        }
        // A module that imports the name in turn, such as a package that re-exports it. Only the
        // longest module is followed, as `package.module.name` reads `name` from `package.module`.
        for &module in modules {
            let name = &parts[*index];
            if let Some(binding) = project.facts[module].references.bindings.get(name.as_str())
                && followed.insert((module, name.clone()))
                && let Some(next) = resolve_import(
                    &project.names[module],
                    binding,
                    &parts[index + 1..],
                    &project.files_by_name,
                )
            {
                pending.push(next);
            }
        }
    }
    found
}

/// Get the directories that a module from outside the project is looked for in.
///
/// A module from outside the project is looked for where the modules of the project are imported from
/// first, as an excluded module is, then in the directories of `PYTHONPATH`, in the site-packages of
/// the environment of the project, if there is one, and then in the standard library.
fn search_path(files: &[Arc<Path>], names: &[ModuleName], python: Option<&Path>) -> Vec<PathBuf> {
    let mut search_path: Vec<PathBuf> = Vec::new();
    let mut seen: FxHashSet<PathBuf> = FxHashSet::default();
    let roots = files
        .iter()
        .zip(names)
        .map(|(path, name)| import_root(path, name));
    // The directories of `PYTHONPATH` come before the installed packages, as Python puts them.
    let python_path: Vec<PathBuf> = std::env::var_os("PYTHONPATH").map_or_default(|paths| {
        std::env::split_paths(&paths)
            .filter(|path| path.is_dir())
            .map(|path| resolve(&path))
            .collect()
    });
    let installed = python.map_or_default(environment_paths);
    for path in roots
        .chain(python_path)
        .chain(installed)
        .chain(standard_library(python))
    {
        if seen.insert(path.clone()) {
            search_path.push(path);
        }
    }
    search_path
}

/// Make the absolute imports of the scripts in a package that only a module beside them can stand
/// for relative imports of it. Python runs a script with its own directory first on the import path,
/// so that `from helpers import name` in a script imports `helpers.py` beside it, and so do the
/// modules beside it that it imports in turn.
///
/// A module of the project or of the environment with the name comes first, as Python finds it when
/// the module is imported rather than run.
fn import_siblings(
    facts: &mut [FileFacts],
    names: &[ModuleName],
    files_by_name: &FxHashMap<String, Vec<usize>>,
    external: &mut ExternalModules,
) -> SiblingImports {
    let package_of = |file: usize| {
        let name = &names[file];
        if name.is_package {
            return None;
        }
        Some(name.name.rsplit_once('.')?.0)
    };
    let mut siblings = SiblingImports {
        roots: (0..facts.len())
            .filter(|&file| facts[file].is_script && package_of(file).is_some())
            .collect(),
        ..SiblingImports::default()
    };
    if siblings.roots.is_empty() {
        return siblings;
    }
    // The names of the modules and packages of the project, with every package above a module.
    let known: FxHashSet<&str> = files_by_name
        .keys()
        .flat_map(|name| {
            name.match_indices('.')
                .map(|(index, _)| &name[..index])
                .chain([name.as_str()])
        })
        .collect();
    let mut pending = siblings.roots.clone();
    let mut seen: FxHashSet<usize> = pending.iter().copied().collect();
    let mut is_elsewhere: FxHashMap<String, bool> = FxHashMap::default();
    while let Some(file) = pending.pop() {
        let Some(package) = package_of(file) else {
            continue;
        };
        let references = &facts[file].references;
        for (module, level, owner) in references
            .imports
            .iter()
            .map(|import| (import.module.as_deref(), import.level, import.owner))
            .chain(
                references
                    .module_imports
                    .iter()
                    .map(|(owner, module)| (Some(module.as_str()), 0, *owner)),
            )
            .chain(references.imported_reads.iter().map(|read| {
                (
                    read.binding.module.as_deref(),
                    read.binding.level,
                    read.owner,
                )
            }))
            .chain(
                references
                    .bindings
                    .values()
                    .map(|binding| (binding.module.as_deref(), binding.level, None)),
            )
        {
            let Some(module) = module.filter(|_| level == 0) else {
                continue;
            };
            let top = module.split('.').next().unwrap_or_default();
            let sibling = format!("{package}.{top}");
            if known.contains(top)
                || !known.contains(sibling.as_str())
                || *is_elsewhere
                    .entry(top.to_string())
                    .or_insert_with(|| external.exists(top))
            {
                continue;
            }
            siblings
                .qualifications
                .entry(file)
                .or_default()
                .insert(module.to_owned());
            for &loaded in files_by_name.get(&sibling).into_iter().flatten() {
                if package_of(loaded).is_some() {
                    siblings
                        .loads
                        .entry((file, owner))
                        .or_default()
                        .insert(loaded);
                    if seen.insert(loaded) {
                        pending.push(loaded);
                    }
                }
            }
        }

        // Declarations also need qualified names for symbol lookup. Finishing that lookup
        // does not execute the imports. The graph follows their owners separately.
        let facts = &mut facts[file];
        for binding in facts
            .references
            .bindings
            .values_mut()
            .chain(facts.lazy.values_mut().flatten())
        {
            if siblings.qualifies(file, binding.module.as_deref(), binding.level) {
                binding.level = 1;
            }
        }
    }
    siblings
}

/// Find the attributes of the classes of a project that inherit from given base classes, such as the
/// fields of the TypedDicts, which are read as keys rather than as attributes, or the members of the
/// enums.
///
/// Returns the attributes of each class that inherits from one of them, directly or not, by its file
/// and its name in the module.
fn subclass_attributes(
    project: &Project,
    hierarchy: &ClassHierarchy,
    bases: &[&str],
) -> FxHashMap<(usize, String), Vec<DefinitionId>> {
    let mut attributes: FxHashMap<(usize, String), Vec<DefinitionId>> = FxHashMap::default();
    for (id, file, definition) in project.definitions_by_file() {
        if definition.kind == Kind::Attribute
            && let Some(class) = &definition.class
        {
            attributes
                .entry((file, class.clone()))
                .or_default()
                .push(id);
        }
    }
    attributes.retain(|(file, class), _| hierarchy.inherits(*file, class, bases));
    attributes
}

/// Find the definitions that count as used whatever references them, and those that are ignored.
///
/// Returns the definitions that count as used by the options or as dunders, and the other definitions
/// that an ignore comment ignores.
fn find_roots(
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
                || definition
                    .class
                    .as_ref()
                    .is_some_and(|class| hierarchy.external_members(file, class).contains(name))
                // A method of a node visitor of `ast`, which its `visit` calls by the name of the node.
                || (definition.kind == Kind::Method
                    && name.starts_with(NODE_VISITOR_PREFIX)
                    && definition
                        .class
                        .as_ref()
                        .is_some_and(|class| hierarchy.inherits(file, class, &NODE_VISITORS)))
                // The objects that the entry points name.
                || entry_objects
                    .get(&file)
                    .is_some_and(|objects| objects.contains(&definition.qualified_name()))
                // What the test frameworks find and call by themselves.
                || rules.uses(definition, file)
                // The options of a definition are those of the project, plus those of every override
                // that covers it.
                || matches_options(definition, &options.used_names, &used_decorators)
                || file_overrides.iter().any(|(_, item, decorators)| {
                    covers(item, file, definition, hierarchy) && matches_options(definition, &item.used_names, decorators)
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

/// Check whether an override that applies to a file covers a definition of it: whether the definition
/// matches the base classes and the class decorators of the override.
fn covers(
    item: &Override,
    file: usize,
    definition: &Definition,
    hierarchy: &ClassHierarchy,
) -> bool {
    // An override with base classes applies to what is defined in a class that inherits from one of
    // them.
    if !item.base_classes.is_empty() {
        let inherits = definition
            .class
            .as_ref()
            .is_some_and(|class| hierarchy.inherits(file, class, &item.base_classes));
        if !inherits {
            return false;
        }
    }
    // An override with class decorators applies to what is defined in a class that has one of them.
    item.class_decorators.is_empty()
        || definition.class.as_ref().is_some_and(|class| {
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

/// Find the unused definitions that only other unused definitions reach.
///
/// An unused function reaches what its code uses, as it would if it were used. An unused class reaches
/// what is defined in it, and an unused function what is defined in it, since deleting them deletes
/// those too. An ignored definition is used, so it reaches nothing here.
fn transitive_definitions(
    project: &Project,
    graph: &Graph,
    unused: &[DefinitionId],
) -> FxHashSet<DefinitionId> {
    let position: FxHashMap<DefinitionId, usize> = unused
        .iter()
        .enumerate()
        .map(|(index, &id)| (id, index))
        .collect();
    let mut successors: Vec<Vec<usize>> = unused
        .iter()
        .map(|&id| {
            graph
                .reached(id)
                .into_iter()
                .filter_map(|other| position.get(&other).copied())
                .collect()
        })
        .collect();

    let mut classes: FxHashMap<(usize, String), usize> = FxHashMap::default();
    // By the line of their name, which is the owner of the code in them.
    let mut functions: FxHashMap<(usize, u32), usize> = FxHashMap::default();
    for (index, &id) in unused.iter().enumerate() {
        let (file, definition) = (&project.definition_files[id], &project.definitions[id]);
        if definition.kind == Kind::Class {
            classes.insert((*file, definition.qualified_name()), index);
        } else if definition.kind.is_function() {
            functions.insert((*file, definition.line), index);
        }
    }
    for (index, &id) in unused.iter().enumerate() {
        let (file, definition) = (&project.definition_files[id], &project.definitions[id]);
        // A definition in a class is in the class first, even if the class is in a function.
        let container = match (&definition.class, definition.owner) {
            (Some(class), _) => classes.get(&(*file, class.clone())),
            (None, Some(function)) => functions.get(&(*file, function)),
            (None, None) => None,
        };
        if let Some(&container) = container {
            successors[container].push(index);
        }
    }

    transitive(&successors)
        .into_iter()
        .enumerate()
        .filter(|(_, reached)| *reached)
        .map(|(index, _)| unused[index])
        .collect()
}

/// Make the findings of a project.
///
/// Returns a finding for each unused definition, for each problem with an ignore comment, and for each
/// file that does not parse, sorted by file, line and column.
fn findings(
    project: &Project,
    discovered: &[Arc<Path>],
    syntax_errors: &FxHashMap<Arc<Path>, Finding>,
    used: &[bool],
    needed: &[DefinitionId],
    is_transitive: &FxHashSet<DefinitionId>,
) -> Vec<Finding> {
    let file_of: FxHashMap<&Arc<Path>, usize> = project
        .files
        .iter()
        .enumerate()
        .map(|(index, path)| (path, index))
        .collect();
    let mut all = Vec::new();
    for path in discovered {
        if let Some(finding) = syntax_errors.get(path) {
            all.push(finding.clone());
            continue;
        }
        // A file that is read and parsed is in the project.
        let file = file_of[path];
        let mut file_findings: Vec<Finding> = project
            .ids_of_file(file)
            .filter(|id| !used[*id])
            .map(|id| {
                let definition = &project.definitions[id];
                Finding {
                    path: definition.path.to_path_buf(),
                    line: definition.line,
                    column: definition.column,
                    code: definition.kind.code(),
                    name: definition.name.clone(),
                    transitive: is_transitive.contains(&id),
                }
            })
            .collect();

        // A code of an ignore comment that ignores nothing is a finding itself.
        let ignored: Vec<(u32, &str)> = needed
            .iter()
            .filter(|id| project.definition_files[**id] == file)
            .map(|id| {
                let definition = &project.definitions[*id];
                (definition.line, definition.kind.code().name())
            })
            .collect();
        let file_facts = &project.facts[file];
        file_findings.extend(apply_ignore_comments(
            path,
            &file_facts.ignore_comments,
            &file_facts.ignore_findings,
            &ignored,
        ));
        file_findings.sort_by_key(|finding| (finding.line, finding.column));
        all.extend(file_findings);
    }
    all
}

/// The definitions of a project and what the code of each owner uses.
///
/// The code of a module runs when it is imported, and the code of a function only when it is called.
/// So a function that is unused uses nothing.
struct Graph<'a> {
    project: &'a Project,
    uses: &'a ProjectUses,
    siblings: &'a SiblingImports,
    active_siblings: Vec<bool>,
    waiting_siblings: FxHashMap<usize, Vec<u32>>,
    /// The members of each enum of the project, by its file and its name in the module, with their
    /// names.
    enums: FxHashMap<(usize, String), (Vec<DefinitionId>, FxHashSet<String>)>,
    by_name: FxHashMap<(usize, Name), Vec<DefinitionId>>,
    by_attribute: FxHashMap<String, Vec<DefinitionId>>,
    by_key: FxHashMap<String, Vec<DefinitionId>>,
    public: FxHashMap<usize, Vec<DefinitionId>>,
    /// The implementations of each abstract method, each with the definition of its class. An
    /// implementation is used once both the abstract method and its class are.
    implementations: FxHashMap<DefinitionId, Vec<(DefinitionId, DefinitionId)>>,
    /// The implementations of used abstract methods, by the class that is not used yet.
    waiting: FxHashMap<DefinitionId, Vec<DefinitionId>>,
    /// What is used so far, and what is left to follow. The ordinary and sibling references of
    /// an owner are each visited at most once.
    used: Vec<bool>,
    pending: Vec<PendingUses>,
    seen: Seen,
}

enum PendingUses {
    Ordinary(usize, Owner),
    Sibling(usize, Owner),
}

/// What the uses of the owners followed so far read, so that each is looked up once.
#[derive(Default)]
struct Seen {
    names: FxHashSet<(usize, Name)>,
    attributes: FxHashSet<Name>,
    keys: FxHashSet<String>,
    objects: FxHashSet<(usize, Vec<Name>)>,
    stars: FxHashSet<usize>,
}

impl<'a> Graph<'a> {
    fn new(
        project: &'a Project,
        uses: &'a ProjectUses,
        siblings: &'a SiblingImports,
        fields: &FxHashSet<DefinitionId>,
        enums: FxHashMap<(usize, String), Vec<DefinitionId>>,
        implementations: FxHashMap<DefinitionId, Vec<(DefinitionId, DefinitionId)>>,
    ) -> Self {
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
            if fields.contains(&id) {
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
        let enums = enums
            .into_iter()
            .map(|(key, members)| {
                let names = members
                    .iter()
                    .map(|id| project.definitions[*id].name.clone())
                    .collect();
                (key, (members, names))
            })
            .collect();

        let mut graph = Self {
            project,
            uses,
            siblings,
            active_siblings: vec![false; project.files.len()],
            waiting_siblings: FxHashMap::default(),
            enums,
            by_name,
            by_attribute,
            by_key,
            public,
            implementations,
            waiting: FxHashMap::default(),
            used: vec![false; project.definitions.len()],
            // Every module's ordinary references are entry points for the check.
            pending: (0..project.files.len())
                .map(|file| PendingUses::Ordinary(file, None))
                .collect(),
            seen: Seen::default(),
        };
        for &file in &siblings.roots {
            graph.activate_siblings(file);
        }
        graph
    }

    fn activate_siblings(&mut self, file: usize) {
        if !self.siblings.qualifications.contains_key(&file)
            || std::mem::replace(&mut self.active_siblings[file], true)
        {
            return;
        }
        self.pending.push(PendingUses::Sibling(file, None));
        if let Some(waiting) = self.waiting_siblings.remove(&file) {
            self.pending.extend(
                waiting
                    .into_iter()
                    .map(|line| PendingUses::Sibling(file, Some(line))),
            );
        }
    }

    /// Use more definitions, and get all the definitions that are used so far: the roots of every call,
    /// and the definitions that the code of the modules uses, directly or through the functions that
    /// it uses.
    ///
    /// The first call also uses what the code of the modules uses. Each later call goes on from where
    /// the one before stopped.
    fn use_definitions(&mut self, roots: impl IntoIterator<Item = DefinitionId>) -> Vec<bool> {
        self.add(roots);
        while let Some(pending) = self.pending.pop() {
            let uses = match pending {
                PendingUses::Ordinary(file, owner) => self.uses.ordinary.get(&(file, owner)),
                PendingUses::Sibling(file, owner) => {
                    for &loaded in self
                        .siblings
                        .loads
                        .get(&(file, owner))
                        .into_iter()
                        .flatten()
                    {
                        self.activate_siblings(loaded);
                    }
                    self.uses.sibling.get(&(file, owner))
                }
            };
            let Some(uses) = uses else {
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
    fn reached(&self, id: DefinitionId) -> FxHashSet<DefinitionId> {
        let (file, definition) = (
            &self.project.definition_files[id],
            &self.project.definitions[id],
        );
        let mut reached = FxHashSet::default();
        if !definition.kind.is_function() {
            return reached;
        }
        let implementations = self.implementations.get(&id).into_iter().flatten();
        reached.extend(implementations.map(|&(_, method)| method));
        let owner = (*file, Some(definition.line));
        if let Some(uses) = self.uses.ordinary.get(&owner) {
            self.look_up(uses, None, &mut reached);
        }
        if self.active_siblings[*file]
            && let Some(uses) = self.uses.sibling.get(&owner)
        {
            self.look_up(uses, None, &mut reached);
        }
        reached
    }

    /// Find the definitions that the uses of an owner can stand for: the names by their module, the
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
            let (file, definition) = (
                &self.project.definition_files[id],
                &self.project.definitions[id],
            );
            if definition.kind.is_function() {
                let line = definition.line;
                self.pending.push(PendingUses::Ordinary(*file, Some(line)));
                if self.active_siblings[*file] {
                    self.pending.push(PendingUses::Sibling(*file, Some(line)));
                } else if self.siblings.qualifications.contains_key(file) {
                    self.waiting_siblings.entry(*file).or_default().push(line);
                }
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
