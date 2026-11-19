//! The unused definitions of a Python project, which are those that nothing references.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use regex::Regex;
use ruff_python_ast::name::Name;
use ruff_python_stdlib::builtins::is_python_builtin;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::classes::{ClassHierarchy, ScriptImports, join};
use crate::config::Config;
use crate::definitions::{Definition, Kind};
use crate::discover::{discover, is_under};
use crate::external::{ExternalModules, environment_paths, standard_library};
use crate::files::{FileFacts, analyze_files};
use crate::frameworks::{FrameworkRules, ProjectFile};
use crate::ignores::{apply_ignore_comments, ignored_codes};
use crate::models::{Finding, Framework, Override, fnmatch};
use crate::modules::{ModuleName, import_root, module_name, resolve};
use crate::references::{Binding, Import, ImportedRead, Owner, Receiver};
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
/// Possible script import contexts and the entry contexts that are always active.
#[derive(Default)]
struct SiblingImports {
    contexts: Vec<SiblingContext>,
    roots: usize,
    qualifications: FxHashMap<usize, FxHashMap<String, Vec<String>>>,
}

/// A module under one script's import directory.
struct SiblingContext {
    file: usize,
    /// Scripts with the same import directory share a scope.
    scope: usize,
    qualifications: FxHashMap<String, String>,
    loads: FxHashMap<Owner, FxHashSet<usize>>,
}

/// References and module loads that apply when a script context and owner are both reachable.
struct SiblingUses {
    file: usize,
    scope: usize,
    has_qualified_imports: bool,
    uses: FxHashMap<Owner, Uses>,
    loads: FxHashMap<Owner, FxHashSet<usize>>,
}

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
    /// Member references that have additional targets when a script supplies their imports.
    scoped_members: FxHashSet<ScopedMember>,
}

#[derive(Eq, Hash, PartialEq)]
enum ScopedMember {
    Named(String, Name),
    Own {
        file: usize,
        class: String,
        name: Name,
        is_super: bool,
    },
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
/// file that does not parse, sorted by file, line and column.
pub(crate) fn find_unused(options: &Config, paths: &[PathBuf]) -> Vec<Finding> {
    let enabled: FxHashSet<Framework> = Framework::ALL
        .into_iter()
        .filter(|framework| options.frameworks.get(framework).copied().unwrap_or(true))
        .collect();

    let discovered: Vec<Arc<Path>> = discover(paths, options)
        .into_iter()
        .map(Arc::from)
        .collect();
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
    let mut definitions = Vec::new();
    let mut definition_files = Vec::new();
    let mut first_definition = Vec::new();
    for (index, file_facts) in facts.iter_mut().enumerate() {
        first_definition.push(definitions.len());
        let file_definitions = std::mem::take(&mut file_facts.definitions);
        definition_files.extend(std::iter::repeat_n(index, file_definitions.len()));
        definitions.extend(file_definitions);
    }
    let mut project = Project {
        files,
        facts,
        names,
        files_by_name,
        definitions,
        definition_files,
        first_definition,
    };
    let mut external = ExternalModules::new(search_path(
        &project.files,
        &project.names,
        options.python.as_deref(),
    ));
    let SiblingImports {
        contexts,
        roots,
        qualifications,
    } = import_siblings(&project, &mut external);
    let modules: Vec<Arc<ModuleSymbols>> = (0..project.files.len())
        .map(|index| {
            let file_facts = &project.facts[index];
            Arc::new(ModuleSymbols::of(
                project.files[index].clone(),
                project.names[index].clone(),
                project.file_definitions(index),
                &file_facts.references,
                file_facts.lazy.clone(),
            ))
        })
        .collect();
    let mut script_imports = ScriptImports::default();
    for context in &contexts {
        for (original, qualified) in &context.qualifications {
            script_imports
                .entry(context.file)
                .or_default()
                .entry(original.clone())
                .or_default()
                .push((context.scope, qualified.clone()));
        }
    }
    if !script_imports.is_empty() {
        for context in &contexts {
            script_imports.entry(context.file).or_default();
        }
    }
    let hierarchy = ClassHierarchy::new(
        modules,
        project.files_by_name.clone(),
        external,
        script_imports,
    );
    let original_imports: FxHashMap<_, _> = qualifications
        .iter()
        .map(|(&file, modules)| {
            (
                file,
                qualify_sibling_symbols(&mut project.facts[file], modules),
            )
        })
        .collect();
    let typed_dicts = subclass_attributes(&project, &hierarchy, &TYPED_DICTS);
    let enums = subclass_attributes(&project, &hierarchy, &ENUMS);
    let fields: FxHashSet<DefinitionId> = typed_dicts.values().flatten().copied().collect();
    let classes = Classes::new(&project, &hierarchy, &fields);
    let typed_dicts: FxHashMap<String, Vec<DefinitionId>> = typed_dicts
        .into_iter()
        .map(|((file, class), fields)| (join(&project.names[file].name, &class), fields))
        .collect();
    let uses = find_uses(
        &project,
        &classes,
        &typed_dicts,
        &original_imports,
        &qualifications,
    );
    let sibling_uses: Vec<_> = contexts
        .into_iter()
        .map(|context| find_sibling_uses(&project, &classes, &original_imports, context))
        .collect();

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

    let mut graph = Graph::new(
        &project,
        &classes,
        &uses,
        &fields,
        enums,
        &sibling_uses,
        roots,
    );

    // An ignore comment is needed for a definition that is unused without it. Such a definition is
    // then used, and so is what it uses.
    let kept_used = graph.use_definitions(kept.iter().copied());
    let needed: Vec<DefinitionId> = ignored.into_iter().filter(|id| !kept_used[*id]).collect();
    let all_used = graph.use_definitions(needed.iter().copied());
    let unused: Vec<DefinitionId> = (0..project.definitions.len())
        .filter(|id| !all_used[*id])
        .collect();
    let is_transitive = transitive_definitions(&project, &graph, &unused);

    let findings = findings(
        &project,
        &discovered,
        &syntax_errors,
        &all_used,
        &needed,
        &is_transitive,
    );

    // The command exits after the check, and the system frees its memory at once, which is faster than
    // dropping the many small parts of these one by one, as clang does too. A caller that checks many
    // times in one process would need them dropped.
    std::mem::forget(graph);
    std::mem::forget(rules);
    std::mem::forget(file_definitions);
    std::mem::forget(uses);
    std::mem::forget(classes);
    std::mem::forget(hierarchy);
    std::mem::forget(sibling_uses);
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
    original_imports: &FxHashMap<usize, FxHashSet<Import>>,
    qualifications: &FxHashMap<usize, FxHashMap<String, Vec<String>>>,
) -> FxHashMap<(usize, Owner), Uses> {
    let mut uses: FxHashMap<(usize, Owner), Uses> = FxHashMap::default();
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
                owner_uses.add_member_of(classes, module, &parts[..=index], attribute, None);
            }
            if read.reads_object {
                owner_uses.objects.insert((file, parts));
            }
        }

        // The original name has no project or external match when it is qualified for a script.
        // Its use is resolved in the script contexts instead of falling back to an unknown receiver.
        let qualified = qualifications.get(&file);
        let is_sibling = |module: Option<&str>, level| {
            level == 0
                && module
                    .is_some_and(|module| qualified.is_some_and(|names| names.contains_key(module)))
        };
        for read in &references.imported_reads {
            if !is_sibling(read.binding.module.as_deref(), read.binding.level) {
                uses.entry((file, read.owner))
                    .or_default()
                    .add_imported_read(project, classes, file, read, None);
            }
        }
        for import in original_imports.get(&file).unwrap_or(&references.imports) {
            if !is_sibling(import.module.as_deref(), import.level)
                && let Some(module) =
                    project.names[file].resolve(import.module.as_deref(), import.level)
                && let Some(files) = project.files_by_name.get(&module)
            {
                uses.entry((file, import.owner))
                    .or_default()
                    .add_import(files, import);
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
            let (found, depends_on_script) =
                classes.read_from_own(file, class, name, is_super, None);
            let owner_uses = uses.entry((file, *owner)).or_default();
            if found.is_some() || !depends_on_script {
                owner_uses.add_member(found, name);
            }
            if depends_on_script {
                owner_uses.scoped_members.insert(ScopedMember::Own {
                    file,
                    class: class.clone(),
                    name: name.clone(),
                    is_super,
                });
            }
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
                .and_then(|mut bindings| bindings.next()?.name.as_deref());
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
    uses
}

/// Resolve the additional references that use this script's import directory.
fn find_sibling_uses(
    project: &Project,
    classes: &Classes,
    original_imports: &FxHashMap<usize, FxHashSet<Import>>,
    context: SiblingContext,
) -> SiblingUses {
    let SiblingContext {
        file,
        scope,
        qualifications,
        loads,
    } = context;
    let mut uses: FxHashMap<Owner, Uses> = FxHashMap::default();
    if !qualifications.is_empty() {
        let references = &project.facts[file].references;
        let qualified = |module: Option<&str>, level| {
            if level == 0 {
                module.and_then(|module| qualifications.get(module))
            } else {
                None
            }
        };
        for read in &references.imported_reads {
            if let Some(module) = qualified(read.binding.module.as_deref(), read.binding.level) {
                let mut read = read.clone();
                read.binding.module = Some(module.clone());
                uses.entry(read.owner).or_default().add_imported_read(
                    project,
                    classes,
                    file,
                    &read,
                    Some(scope),
                );
            }
        }
        for import in &original_imports[&file] {
            if let Some(module) = qualified(import.module.as_deref(), import.level)
                && let Some(files) = project.files_by_name.get(module)
            {
                uses.entry(import.owner)
                    .or_default()
                    .add_import(files, import);
            }
        }
    }
    SiblingUses {
        file,
        scope,
        has_qualified_imports: !qualifications.is_empty(),
        uses,
        loads,
    }
}

impl Uses {
    fn add_import(&mut self, files: &[usize], import: &Import) {
        for &imported in files {
            if import.name == "*" {
                self.stars.insert(imported);
            } else {
                self.names.insert((imported, Name::new(&import.name)));
            }
        }
    }

    fn add_imported_read(
        &mut self,
        project: &Project,
        classes: &Classes,
        file: usize,
        read: &ImportedRead,
        scope: Option<usize>,
    ) {
        let Some(resolved) = resolve_import(
            &project.names[file],
            &read.binding,
            &read.attributes,
            &project.files_by_name,
        ) else {
            self.attributes
                .extend(read.attributes.iter().chain(&read.called).cloned());
            return;
        };
        if read.reads_object || !read.attributes.is_empty() {
            let objects = imported_objects(&resolved, project);
            if !read.attributes.is_empty() {
                // An imported module can be reached through a re-export. Its attributes
                // are names of the original module, just as they are for a direct import.
                self.names.extend(
                    objects
                        .iter()
                        .map(|(file, parts)| (*file, parts[0].clone())),
                );
            }
            if read.reads_object {
                self.objects.extend(objects);
            }
        }
        if let Some(called) = &read.called {
            self.add_member_of(classes, "", &resolved.0, called, scope);
        }
        if read.attributes.is_empty() {
            return;
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
            self.add_member_of(classes, "", &parts[..index], &parts[index], scope);
        }
    }

    /// Add a read of a member from a chain of names, such as `name` from `package.module.Class`.
    ///
    /// `module` is the module that the chain is written in, or empty for a chain that starts with
    /// the name of a module.
    fn add_member_of(
        &mut self,
        classes: &Classes,
        module: &str,
        chain: &[Name],
        name: &Name,
        scope: Option<usize>,
    ) {
        let written = join(module, &chain.join("."));
        let (found, depends_on_script) = classes.read_from(&written, name, scope);
        if found.is_some() || !depends_on_script {
            self.add_member(found, name);
        }
        if scope.is_none() && depends_on_script {
            self.scoped_members
                .insert(ScopedMember::Named(written, name.clone()));
        }
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
    dotted_names: Vec<String>,
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
    /// Whether choosing the base classes used in a self or super read needs a script's imports.
    scoped_lineages: Vec<OnceCell<bool>>,
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
        let mut dotted_names = Vec::new();
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
                if hierarchy.has_script_imports() {
                    dotted_names.push(dotted.clone());
                }
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
            dotted_names,
            names,
            ancestors,
            lineages: std::iter::repeat_with(Default::default)
                .take(descendants.len())
                .collect(),
            scoped_lineages: if hierarchy.has_script_imports() {
                std::iter::repeat_with(OnceCell::new)
                    .take(descendants.len())
                    .collect()
            } else {
                Vec::new()
            },
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
    fn read_from(
        &self,
        written: &str,
        name: &str,
        scope: Option<usize>,
    ) -> (Option<Vec<DefinitionId>>, bool) {
        let last = written.rsplit('.').next().unwrap_or(written);
        if !self.names.contains(last) && !self.hierarchy.can_reexport(written) {
            return (None, false);
        }
        let followed = self.hierarchy.follow_name(written, scope);
        let classes = followed
            .names
            .iter()
            .flat_map(|dotted| self.by_dotted.get(dotted).into_iter().flatten().copied());
        (self.members(classes, name), followed.depends_on_script)
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
        scope: Option<usize>,
    ) -> (Option<Vec<DefinitionId>>, bool) {
        let Some(&id) = self.ids.get(&(file, class.to_string())) else {
            return (None, false);
        };
        if self.metaclasses[id] || !self.by_member.contains_key(name) {
            return (None, false);
        }
        if self
            .scoped_lineages
            .get(id)
            .is_some_and(|depends_on_script| {
                *depends_on_script.get_or_init(|| {
                    std::iter::once(&id)
                        .chain(&self.descendants[id])
                        .any(|&class| {
                            self.hierarchy
                                .follow_name(&self.dotted_names[class], None)
                                .depends_on_script
                        })
                })
            })
        {
            return (
                self.members(self.scoped_lineage(id, skip_own, scope), name),
                true,
            );
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
        (self.members(lineage.iter().copied(), name), false)
    }

    fn scoped_lineage(
        &self,
        id: ClassId,
        skip_own: bool,
        scope: Option<usize>,
    ) -> FxHashSet<ClassId> {
        let mut lineage = FxHashSet::default();
        let mut instances = Vec::new();
        for &instance in std::iter::once(&id).chain(&self.descendants[id]) {
            let followed = self
                .hierarchy
                .follow_name(&self.dotted_names[instance], scope);
            if instance == id || followed.names.contains(&self.dotted_names[id]) {
                instances.push(instance);
                lineage.insert(instance);
                for dotted in &followed.names {
                    lineage.extend(self.by_dotted.get(dotted).into_iter().flatten().copied());
                }
            }
        }
        if skip_own {
            for instance in instances {
                lineage.remove(&instance);
            }
        }
        lineage
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
            if let Some(bindings) = project.facts[module].references.bindings.get(name.as_str())
                && followed.insert((module, name.clone()))
            {
                pending.extend(bindings.filter_map(|binding| {
                    resolve_import(
                        &project.names[module],
                        binding,
                        &parts[index + 1..],
                        &project.files_by_name,
                    )
                }));
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

/// Qualify absolute imports that resolve beside a script. Python runs a script with its directory
/// first on the import path, and that directory is still on the path when a sibling package or one
/// of its submodules imports another sibling.
///
/// A module of the project or of the environment with the name comes first, as Python finds it when
/// the module is imported rather than run.
fn import_siblings(project: &Project, external: &mut ExternalModules) -> SiblingImports {
    let names = &project.names;
    let facts = &project.facts;
    let files_by_name = &project.files_by_name;
    let mut imports = SiblingImports::default();
    let mut modules: Vec<(usize, &str)> = names
        .iter()
        .enumerate()
        .filter(|(file, name)| facts[*file].is_script && !name.is_package)
        .filter_map(|(file, name)| Some((file, name.name.rsplit_once('.')?.0)))
        .collect();
    if modules.is_empty() {
        return imports;
    }
    imports.roots = modules.len();
    // The names of the modules and packages of the project, with every package above a module.
    let known: FxHashSet<&str> = files_by_name
        .keys()
        .flat_map(|name| {
            name.match_indices('.')
                .map(|(index, _)| &name[..index])
                .chain([name.as_str()])
        })
        .collect();
    let mut indices: FxHashMap<_, _> = modules.iter().copied().zip(0..).collect();
    let mut is_elsewhere: FxHashMap<String, bool> = FxHashMap::default();
    let mut scopes = FxHashMap::default();
    let mut flat_loads = Vec::new();
    while let Some(&(file, package)) = modules.get(imports.contexts.len()) {
        let next_scope = scopes.len();
        let mut context = SiblingContext {
            file,
            scope: *scopes.entry(package).or_insert(next_scope),
            qualifications: FxHashMap::default(),
            loads: FxHashMap::default(),
        };
        let mut siblings: FxHashSet<(Owner, String)> = FxHashSet::default();
        let mut flat_modules = FxHashSet::default();
        let mut visit_import = |module: Option<&str>,
                                level,
                                imported: Option<&str>,
                                owner: Owner,
                                known_to_flat_walk| {
            let qualified = if level == 0 {
                let Some(module) = module else { return };
                let top = module.split('.').next().unwrap_or_default();
                let sibling = format!("{package}.{top}");
                if known.contains(top)
                    || !known.contains(sibling.as_str())
                    || *is_elsewhere
                        .entry(top.to_string())
                        .or_insert_with(|| external.exists(top))
                {
                    return;
                }
                format!("{package}.{module}")
            } else if let Some(resolved) = names[file].resolve(module, level) {
                resolved
            } else {
                return;
            };
            if !qualified
                .strip_prefix(package)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
            {
                return;
            }
            if level == 0
                && let Some(module) = module
            {
                if known_to_flat_walk && !module.contains('.') {
                    flat_modules.insert(qualified.clone());
                }
                context
                    .qualifications
                    .insert(module.to_string(), qualified.clone());
                let targets = imports
                    .qualifications
                    .entry(file)
                    .or_default()
                    .entry(module.to_string())
                    .or_default();
                if !targets.contains(&qualified) {
                    targets.push(qualified.clone());
                }
            }

            // An explicit relative import can load another module of the same package. Keep the
            // script's search directory for that module too.
            if let Some(name) = imported.filter(|name| *name != "*") {
                let child = join(&qualified, name);
                if files_by_name.contains_key(&child)
                    && !files_by_name
                        .get(&qualified)
                        .into_iter()
                        .flatten()
                        .any(|&file| {
                            let facts = &facts[file];
                            facts.lazy.contains_key(name)
                                || facts.references.bindings.primary.get(name).is_some_and(
                                    |binding| {
                                        // `from . import child` in the package initializer loads that child.
                                        binding.name.as_deref() != Some(name)
                                            || names[file]
                                                .resolve(binding.module.as_deref(), binding.level)
                                                .as_deref()
                                                != Some(qualified.as_str())
                                    },
                                )
                                || project.file_definitions(file).iter().any(|definition| {
                                    definition.owner.is_none()
                                        && definition.class.is_none()
                                        && definition.name == name
                                })
                        })
                {
                    // A from-import tries a submodule only if the package lacks the attribute.
                    siblings.insert((owner, child));
                }
            }
            siblings.insert((owner, qualified));
        };
        let file_facts = &facts[file];
        let references = &file_facts.references;
        for (owner, module) in &references.module_imports {
            visit_import(
                Some(module.as_str()),
                0,
                None,
                *owner,
                !matches!(owner, Owner::Function(_)),
            );
        }
        for import in &references.imports {
            visit_import(
                import.module.as_deref(),
                import.level,
                Some(&import.name),
                import.owner,
                true,
            );
        }
        for read in &references.imported_reads {
            let binding = &read.binding;
            visit_import(
                binding.module.as_deref(),
                binding.level,
                binding.name.as_deref(),
                read.owner,
                true,
            );
        }

        let mut flat = FxHashSet::default();
        for (owner, sibling) in siblings {
            // A dotted import loads the package initializers along its path. The script's own
            // package is outside the import as it was written, so start below it.
            let Some(relative) = sibling
                .strip_prefix(package)
                .and_then(|rest| rest.strip_prefix('.'))
            else {
                continue;
            };
            let start = package.len() + 1;
            let ends = relative
                .match_indices('.')
                .map(|(end, _)| start + end)
                .chain([sibling.len()]);
            for end in ends {
                for &imported in files_by_name.get(&sibling[..end]).into_iter().flatten() {
                    let loaded = *indices.entry((imported, package)).or_insert_with(|| {
                        let index = modules.len();
                        modules.push((imported, package));
                        index
                    });
                    context.loads.entry(owner).or_default().insert(loaded);
                    if end == sibling.len()
                        && flat_modules.contains(&sibling)
                        && !names[imported].is_package
                    {
                        flat.insert(loaded);
                    }
                }
            }
        }
        flat_loads.push(flat);
        imports.contexts.push(context);
    }

    // The existing flat sibling walk counts loads without considering the containing function.
    // Keep that behavior for modules it could already reach. Loads discovered by visiting a
    // dotted import need a live owner before their module runs.
    let mut pending: Vec<_> = (0..imports.roots).collect();
    let mut seen: FxHashSet<_> = pending.iter().copied().collect();
    while let Some(context) = pending.pop() {
        let loads = &flat_loads[context];
        if !loads.is_empty() {
            imports.contexts[context]
                .loads
                .entry(Owner::Module)
                .or_default()
                .extend(loads);
            pending.extend(loads.iter().copied().filter(|loaded| seen.insert(*loaded)));
        }
    }
    imports
}

/// Qualify sibling bindings for symbol lookup and keep the raw imports for reachability.
fn qualify_sibling_symbols(
    file_facts: &mut FileFacts,
    qualifications: &FxHashMap<String, Vec<String>>,
) -> FxHashSet<Import> {
    let modules = |module: Option<&str>, level| -> &[String] {
        if level != 0 {
            return &[];
        }
        module
            .and_then(|module| qualifications.get(module))
            .map_or(&[], Vec::as_slice)
    };
    let references = &mut file_facts.references;
    let original = std::mem::take(&mut references.imports);
    for import in &original {
        if let Some((first, rest)) = modules(import.module.as_deref(), import.level).split_first() {
            for module in std::iter::once(first).chain(rest) {
                let mut alternative = import.clone();
                alternative.module = Some(module.clone());
                references.imports.insert(alternative);
            }
        } else {
            references.imports.insert(import.clone());
        }
    }
    let qualify = |binding: &mut Binding| {
        let Some((first, rest)) = modules(binding.module.as_deref(), binding.level).split_first()
        else {
            return Vec::new();
        };
        binding.module = Some(first.clone());
        rest.iter()
            .map(|module| Binding {
                module: Some(module.clone()),
                ..binding.clone()
            })
            .collect()
    };
    for (name, binding) in &mut references.bindings.primary {
        let alternatives = qualify(binding);
        if !alternatives.is_empty() {
            references
                .bindings
                .alternatives
                .insert(name.clone(), alternatives);
        }
    }
    for bindings in file_facts.lazy.values_mut() {
        let alternatives: Vec<_> = bindings.iter_mut().flat_map(qualify).collect();
        bindings.extend(alternatives);
    }
    original
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
    classes: &'a Classes<'a>,
    uses: &'a FxHashMap<(usize, Owner), Uses>,
    sibling_uses: &'a [SiblingUses],
    contexts_by_file: FxHashMap<usize, Vec<usize>>,
    contexts_by_scope: FxHashMap<(usize, usize), usize>,
    scoped_files: FxHashSet<usize>,
    contexts: Vec<ContextReachability>,
    root_functions: FxHashMap<usize, Vec<DefinitionId>>,
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
    /// The implementations of used abstract methods with the scopes that reached them, by the
    /// class that is not used yet.
    waiting: FxHashMap<DefinitionId, Vec<(Option<usize>, DefinitionId)>>,
    /// A definition reached in any scope is used. Each function body is visited in every scope
    /// that reaches it so its imports can resolve with that scope's search directory.
    used: Vec<bool>,
    ordinary_used: Vec<bool>,
    ordinary_seen: Seen,
    scripts: FxHashMap<usize, ScriptReachability>,
    pending: Vec<PendingUses>,
}

#[derive(Default)]
struct ContextReachability {
    active: bool,
    imported: bool,
    waiting: Vec<u32>,
}

#[derive(Default)]
struct ScriptReachability {
    used: FxHashSet<DefinitionId>,
    seen: Seen,
}

enum PendingUses {
    /// Ordinary references, with the caller's script scope when there is one.
    File(usize, Owner, Option<usize>),
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
        classes: &'a Classes<'a>,
        uses: &'a FxHashMap<(usize, Owner), Uses>,
        fields: &FxHashSet<DefinitionId>,
        enums: FxHashMap<(usize, String), Vec<DefinitionId>>,
        sibling_uses: &'a [SiblingUses],
        script_roots: usize,
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

        let mut contexts_by_file: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
        let mut contexts_by_scope = FxHashMap::default();
        // A scope without sibling imports can use the ordinary traversal. In particular, scripts
        // that share a library should only walk it once when their import directories add nothing.
        let qualified_scopes: FxHashSet<usize> = sibling_uses
            .iter()
            .filter(|context| context.has_qualified_imports)
            .map(|context| context.scope)
            .collect();
        let mut scoped_script_files = FxHashSet::default();
        let mut scoped_entries = Vec::new();
        for (index, context) in sibling_uses
            .iter()
            .enumerate()
            .filter(|(_, context)| qualified_scopes.contains(&context.scope))
        {
            contexts_by_file
                .entry(context.file)
                .or_default()
                .push(index);
            contexts_by_scope.insert((context.file, context.scope), index);
            if index < script_roots {
                scoped_script_files.insert(context.file);
                scoped_entries.push(index);
            }
        }
        let mut pending = Vec::new();
        // Every checked module contributes its top-level code.
        for (file, facts) in project.facts.iter().enumerate() {
            pending.push(PendingUses::File(file, Owner::Module, None));
            if facts.is_script {
                pending.push(PendingUses::File(file, Owner::Imported, None));
                if !scoped_script_files.contains(&file) {
                    pending.push(PendingUses::File(file, Owner::Script, None));
                }
            }
        }
        let mut graph = Self {
            project,
            classes,
            uses,
            sibling_uses,
            contexts_by_file,
            contexts_by_scope,
            scoped_files: FxHashSet::default(),
            contexts: (0..sibling_uses.len())
                .map(|_| ContextReachability::default())
                .collect(),
            root_functions: FxHashMap::default(),
            enums,
            by_name,
            by_attribute,
            by_key,
            public,
            implementations: classes.implementations(project),
            waiting: FxHashMap::default(),
            used: vec![false; project.definitions.len()],
            ordinary_used: vec![false; project.definitions.len()],
            ordinary_seen: Seen::default(),
            scripts: FxHashMap::default(),
            pending,
        };
        if !scoped_entries.is_empty() {
            graph.scoped_files = graph.files_requiring_scopes();
        }
        for script in scoped_entries {
            graph.activate_context(script);
            graph.visit_context_owner(script, Owner::Script);
        }
        graph
    }

    /// A script context is needed only where the ordinary call graph can reach a scoped import
    /// or member. Code outside this set shares the ordinary reachability traversal.
    fn files_requiring_scopes(&self) -> FxHashSet<usize> {
        let mut scoped: FxHashSet<_> = self.contexts_by_file.keys().copied().collect();
        let mut callers: FxHashMap<usize, FxHashSet<usize>> = FxHashMap::default();
        let mut found = Vec::new();
        for (&(file, _), uses) in self.uses {
            if !uses.scoped_members.is_empty() {
                scoped.insert(file);
            }
            self.look_up(uses, None, None, &mut found);
            for id in found.drain(..) {
                let target = self.project.definition_files[id];
                if target != file {
                    callers.entry(target).or_default().insert(file);
                }
            }
        }
        for (&id, methods) in &self.implementations {
            let file = self.project.definition_files[id];
            for &(_, method) in methods {
                let target = self.project.definition_files[method];
                if target != file {
                    callers.entry(target).or_default().insert(file);
                }
            }
        }
        let mut pending: Vec<_> = scoped.iter().copied().collect();
        while let Some(file) = pending.pop() {
            for &caller in callers.get(&file).into_iter().flatten() {
                if scoped.insert(caller) {
                    pending.push(caller);
                }
            }
        }
        scoped
    }

    /// Visit the module and any functions already reached in this script scope.
    fn activate_context(&mut self, context: usize) {
        let state = &mut self.contexts[context];
        if state.active {
            return;
        }
        state.active = true;
        self.pending.extend(
            state
                .waiting
                .drain(..)
                .map(|line| PendingUses::Sibling(context, Owner::Function(line))),
        );
        self.visit_context_owner(context, Owner::Module);
        let SiblingUses { file, scope, .. } = self.sibling_uses[context];
        let roots = self.root_functions.get(&file).cloned().unwrap_or_default();
        self.add(roots.into_iter().map(|id| (Some(scope), id)));
    }

    fn visit_context_owner(&mut self, context: usize, owner: Owner) {
        let SiblingUses { file, scope, .. } = self.sibling_uses[context];
        self.pending
            .push(PendingUses::File(file, owner, Some(scope)));
        self.pending.push(PendingUses::Sibling(context, owner));
    }

    /// Use more definitions, and get all the definitions that are used so far: the roots of every call,
    /// and the definitions that the code of the modules uses, directly or through the functions that
    /// it uses.
    ///
    /// The first call also uses what the code of the modules uses. Each later call goes on from where
    /// the one before stopped.
    fn use_definitions(&mut self, roots: impl IntoIterator<Item = DefinitionId>) -> Vec<bool> {
        let mut initial = Vec::new();
        for id in roots {
            initial.push((None, id));
            let file = self.project.definition_files[id];
            if self.project.definitions[id].kind.is_function()
                && let Some(contexts) = self.contexts_by_file.get(&file)
            {
                // Configured and implicit roots have no caller. They can run in any scope
                // that loads their module.
                self.root_functions.entry(file).or_default().push(id);
                initial.extend(contexts.iter().filter_map(|&context| {
                    self.contexts[context]
                        .active
                        .then_some((Some(self.sibling_uses[context].scope), id))
                }));
            }
        }
        self.add(initial);
        while let Some(pending) = self.pending.pop() {
            let (scope, uses) = match pending {
                PendingUses::File(file, owner, scope) => (scope, self.uses.get(&(file, owner))),
                PendingUses::Sibling(context, owner) => {
                    let sibling = &self.sibling_uses[context];
                    for &loaded in sibling.loads.get(&owner).into_iter().flatten() {
                        self.activate_context(loaded);
                        if self.project.facts[self.sibling_uses[loaded].file].is_script
                            && !std::mem::replace(&mut self.contexts[loaded].imported, true)
                        {
                            self.visit_context_owner(loaded, Owner::Imported);
                        }
                    }
                    (Some(sibling.scope), sibling.uses.get(&owner))
                }
            };
            let Some(uses) = uses else {
                continue;
            };
            let mut seen = std::mem::take(self.seen(scope));
            let mut found: Vec<DefinitionId> = Vec::new();
            self.look_up(uses, scope, Some(&mut seen), &mut found);
            *self.seen(scope) = seen;
            self.add(found.into_iter().map(|id| (scope, id)));
        }
        self.used.clone()
    }

    fn seen(&mut self, scope: Option<usize>) -> &mut Seen {
        match scope {
            Some(scope) => &mut self.scripts.entry(scope).or_default().seen,
            None => &mut self.ordinary_seen,
        }
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
        let owner = Owner::Function(definition.line);
        if let Some(uses) = self.uses.get(&(*file, owner)) {
            self.look_up(uses, None, None, &mut reached);
        }
        for &context in self.contexts_by_file.get(file).into_iter().flatten() {
            if self.contexts[context].active {
                let sibling = &self.sibling_uses[context];
                if let Some(uses) = self.uses.get(&(*file, owner)) {
                    self.look_up_scoped_members(uses, sibling.scope, &mut reached);
                }
                if let Some(uses) = sibling.uses.get(&owner) {
                    self.look_up(uses, Some(sibling.scope), None, &mut reached);
                }
            }
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
        scope: Option<usize>,
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
        if let Some(scope) = scope {
            self.look_up_scoped_members(uses, scope, found);
        }
    }

    fn look_up_scoped_members(
        &self,
        uses: &Uses,
        scope: usize,
        found: &mut impl Extend<DefinitionId>,
    ) {
        for member in &uses.scoped_members {
            let (definitions, _) = match member {
                ScopedMember::Named(written, name) => {
                    self.classes.read_from(written, name, Some(scope))
                }
                ScopedMember::Own {
                    file,
                    class,
                    name,
                    is_super,
                } => self
                    .classes
                    .read_from_own(*file, class, name, *is_super, Some(scope)),
            };
            found.extend(definitions.into_iter().flatten());
        }
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
    fn add(&mut self, definitions: impl IntoIterator<Item = (Option<usize>, DefinitionId)>) {
        let mut queue: Vec<_> = definitions.into_iter().collect();
        while let Some((scope, id)) = queue.pop() {
            let file = self.project.definition_files[id];
            let scope = scope.filter(|_| self.scoped_files.contains(&file));
            let first_use = match scope {
                Some(scope) => self.scripts.entry(scope).or_default().used.insert(id),
                None => !std::mem::replace(&mut self.ordinary_used[id], true),
            };
            if !first_use {
                continue;
            }
            self.used[id] = true;
            let definition = &self.project.definitions[id];
            if definition.kind.is_function() {
                let owner = Owner::Function(definition.line);
                self.pending.push(PendingUses::File(file, owner, scope));
                if let Some(scope) = scope
                    && let Some(&context) = self.contexts_by_scope.get(&(file, scope))
                {
                    let state = &mut self.contexts[context];
                    if state.active {
                        self.pending.push(PendingUses::Sibling(context, owner));
                    } else {
                        state.waiting.push(definition.line);
                    }
                }
            }
            for &(class, method) in self.implementations.get(&id).into_iter().flatten() {
                if self.used[class] {
                    queue.push((scope, method));
                } else {
                    self.waiting.entry(class).or_default().push((scope, method));
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
