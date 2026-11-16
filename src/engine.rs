//! The unused definitions of a Python project, which are those that nothing references.

use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::classes::ClassHierarchy;
use crate::definitions::{Definition, Kind};
use crate::discover::{discover, is_under};
use crate::external::{ExternalModules, environment_paths, standard_library};
use crate::files::{FileFacts, analyze_files};
use crate::frameworks::FrameworkRules;
use crate::ignores::{apply_ignore_comments, ignored_codes};
use crate::models::{Finding, Framework, Override, fnmatch};
use crate::modules::{ModuleName, import_root, module_name, resolve};
use crate::references::{Binding, Owner};
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

/// A definition of the project, by its index among the definitions of every file in order.
type DefinitionId = usize;

/// The options of a check.
pub(crate) struct Options {
    /// The files and directories that make up the project.
    pub paths: Vec<PathBuf>,
    /// The names, as patterns, of the definitions that count as used.
    pub used_names: Vec<String>,
    /// The decorators that make the definitions they decorate count as used.
    pub used_decorators: Vec<String>,
    /// The patterns of the files and directories that are not part of the project.
    pub exclude: Vec<String>,
    /// The options that apply only to some of the files of the project.
    pub overrides: Vec<Override>,
    /// The Python environment or interpreter of the project, if there is one.
    pub python: Option<PathBuf>,
    /// The pattern of the names of the variables that are not used on purpose.
    pub dummy_variable_rgx: String,
    /// The object references of the entry points of the project.
    pub entry_points: Vec<String>,
    /// Whether the rules of each framework apply, over the defaults.
    pub frameworks: FxHashMap<Framework, bool>,
}

/// What the code of an owner uses.
#[derive(Default)]
struct Uses {
    /// The names of the modules of the project that the code uses, each with its module.
    names: FxHashSet<(usize, String)>,
    /// The names that the code reads as attributes of an object.
    attributes: FxHashSet<String>,
    /// The string keys that the code reads from a mapping.
    keys: FxHashSet<String>,
    /// The objects of the modules of the project that the code reads, each with its module and its
    /// name in the module followed by the attributes that are read from it, in order.
    objects: FxHashSet<(usize, Vec<String>)>,
    /// The modules that the code star imports.
    stars: FxHashSet<usize>,
    /// The functions and classes that are local variables of a function, that the code reads.
    definitions: FxHashSet<DefinitionId>,
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

/// Find the unused definitions of a Python project, which are those that nothing references.
///
/// Returns a finding for each unused definition, for each problem with an ignore comment, and for each
/// file that does not parse, sorted by file, line and column.
pub(crate) fn find_unused(options: &Options) -> Vec<Finding> {
    let enabled: FxHashSet<Framework> = Framework::ALL
        .into_iter()
        .filter(|framework| options.frameworks.get(framework).copied().unwrap_or(true))
        .collect();

    let discovered: Vec<Arc<Path>> = discover(&options.paths, &options.exclude)
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
    let roots: Vec<PathBuf> = options
        .paths
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
    let project = Project {
        files,
        facts,
        names,
        files_by_name,
        definitions,
        definition_files,
        first_definition,
    };

    let uses = find_uses(&project);
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
    let hierarchy = make_hierarchy(&project, modules, options.python.as_deref());
    let typed_dicts = subclass_attributes(&project, &hierarchy, &TYPED_DICTS);
    let enums = subclass_attributes(&project, &hierarchy, &ENUMS);

    let module_names: Vec<String> = project.names.iter().map(|name| name.name.clone()).collect();
    let file_definitions: Vec<(&Path, &[Definition])> = (0..project.files.len())
        .map(|index| (&*project.files[index], project.file_definitions(index)))
        .collect();
    let rules = FrameworkRules::new(&enabled, &file_definitions, &module_names, &hierarchy);
    let (kept, ignored) = find_roots(&project, &hierarchy, options, &rules);

    let fields: FxHashSet<DefinitionId> = typed_dicts.values().flatten().copied().collect();
    let mut graph = Graph::new(&project, &uses, &fields, enums);

    // An ignore comment is needed for a definition that is unused without it. Such a definition is
    // then used, and so is what it uses.
    let kept_used = graph.use_definitions(kept.iter().copied());
    let needed: Vec<DefinitionId> = ignored.into_iter().filter(|id| !kept_used[*id]).collect();
    let all_used = graph.use_definitions(needed.iter().copied());
    let unused: Vec<DefinitionId> = (0..project.definitions.len())
        .filter(|id| !all_used[*id])
        .collect();
    let is_transitive = transitive_definitions(&project, &graph, &unused);

    findings(
        &project,
        &discovered,
        &syntax_errors,
        &all_used,
        &needed,
        &is_transitive,
    )
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
}

/// Find what the code of each owner uses.
///
/// A plain name refers to the module that it is read in, an imported name to the module that it is
/// imported from, and an attribute of an object is matched by name only.
fn find_uses(project: &Project) -> FxHashMap<(usize, Owner), Uses> {
    let mut uses: FxHashMap<(usize, Owner), Uses> = FxHashMap::default();
    for (file, file_facts) in project.facts.iter().enumerate() {
        let references = &file_facts.references;
        for read in &references.reads {
            let owner_uses = uses.entry((file, read.owner)).or_default();
            owner_uses.names.insert((file, read.root.clone()));
            owner_uses
                .attributes
                .extend(read.attributes.iter().cloned());
            if read.reads_object {
                let mut parts = vec![read.root.clone()];
                parts.extend(read.attributes.iter().cloned());
                owner_uses.objects.insert((file, parts));
            }
        }

        for read in &references.imported_reads {
            let owner_uses = uses.entry((file, read.owner)).or_default();
            let Some(resolved) = resolve_import(
                &project.names[file],
                &read.binding,
                &read.attributes,
                &project.files_by_name,
            ) else {
                owner_uses
                    .attributes
                    .extend(read.attributes.iter().cloned());
                continue;
            };
            if read.reads_object {
                owner_uses
                    .objects
                    .extend(imported_objects(&resolved, project));
            }
            if read.attributes.is_empty() {
                continue;
            }
            // An attribute of an imported name is resolved: `package.module.name` is `name` in
            // `package.module`.
            let (parts, starts) = &resolved;
            for (index, modules) in starts {
                owner_uses.names.extend(
                    modules
                        .iter()
                        .map(|&module| (module, parts[*index].clone())),
                );
            }
            // What follows the last name that a module of the project holds is read from an object.
            let first_of_object = starts
                .iter()
                .map(|(index, _)| index + 1)
                .chain([parts.len() - read.attributes.len()])
                .max()
                .unwrap_or(0);
            owner_uses
                .attributes
                .extend(parts[first_of_object..].iter().cloned());
        }

        for import in &references.imports {
            let module = project.names[file]
                .resolve(import.module.as_deref(), import.level)
                .unwrap_or_default();
            for &imported in project.files_by_name.get(&module).into_iter().flatten() {
                let owner_uses = uses.entry((file, import.owner)).or_default();
                if import.name == "*" {
                    owner_uses.stars.insert(imported);
                } else {
                    owner_uses.names.insert((imported, import.name.clone()));
                }
            }
        }

        for (owner, name) in &references.attributes {
            uses.entry((file, *owner))
                .or_default()
                .attributes
                .insert(name.clone());
        }
        for (owner, name) in &references.keys {
            uses.entry((file, *owner))
                .or_default()
                .keys
                .insert(name.clone());
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

/// The dotted name that an imported name and the attributes read from it stand for, as its parts,
/// and each shorter start of it that is a module of the project, shortest first, as its number of
/// parts with its files.
type Resolved = (Vec<String>, Vec<(usize, Vec<usize>)>);

/// Find the dotted name that an imported name, and the attributes read from it, stand for.
///
/// Returns None if it is imported from beyond the root.
fn resolve_import(
    module: &ModuleName,
    binding: &Binding,
    attributes: &[String],
    files_by_name: &FxHashMap<String, Vec<usize>>,
) -> Option<Resolved> {
    let resolved = module.resolve(binding.module.as_deref(), binding.level)?;
    let mut parts: Vec<String> = resolved.split('.').map(ToString::to_string).collect();
    if let Some(name) = binding.name.as_ref().filter(|name| !name.is_empty()) {
        parts.push(name.clone());
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
fn imported_objects(resolved: &Resolved, project: &Project) -> FxHashSet<(usize, Vec<String>)> {
    let mut found = FxHashSet::default();
    // Each binding that is followed is followed once, which ends a cycle of modules that import the
    // names of each other.
    let mut followed: FxHashSet<(usize, String)> = FxHashSet::default();
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
            if let Some(binding) = project.facts[module].references.bindings.get(name)
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

/// Build the hierarchy of the classes of a project.
///
/// A module from outside the project is looked for where the modules of the project are imported from
/// first, as an excluded module is, then in the directories of `PYTHONPATH`, in the site-packages of
/// the environment of the project, if there is one, and then in the standard library.
fn make_hierarchy(
    project: &Project,
    modules: Vec<Arc<ModuleSymbols>>,
    python: Option<&Path>,
) -> ClassHierarchy {
    let mut search_path: Vec<PathBuf> = Vec::new();
    let mut seen: FxHashSet<PathBuf> = FxHashSet::default();
    let roots = project
        .files
        .iter()
        .zip(&project.names)
        .map(|(path, name)| import_root(path, name));
    // The directories of `PYTHONPATH` come before the installed packages, as Python puts them.
    let python_path: Vec<PathBuf> = std::env::var_os("PYTHONPATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .filter(|path| path.is_dir())
                .map(|path| resolve(&path))
                .collect()
        })
        .unwrap_or_default();
    let installed = python.map(environment_paths).unwrap_or_default();
    for path in roots
        .chain(python_path)
        .chain(installed)
        .chain(standard_library(python))
    {
        if seen.insert(path.clone()) {
            search_path.push(path);
        }
    }
    ClassHierarchy::new(
        modules,
        project.files_by_name.clone(),
        ExternalModules::new(search_path),
    )
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
    for (id, (file, definition)) in project
        .definition_files
        .iter()
        .zip(&project.definitions)
        .enumerate()
    {
        if definition.kind == Kind::Attribute
            && let Some(owner) = &definition.owner
        {
            attributes
                .entry((*file, owner.clone()))
                .or_default()
                .push(id);
        }
    }
    attributes.retain(|(file, owner), _| {
        let ancestors = hierarchy.ancestors(*file, owner);
        bases.iter().any(|base| ancestors.contains(*base))
    });
    attributes
}

/// Find the definitions that count as used whatever references them, and those that are ignored.
///
/// Returns the definitions that count as used by the options or as dunders, and the other definitions
/// that an ignore comment ignores.
fn find_roots(
    project: &Project,
    hierarchy: &ClassHierarchy,
    options: &Options,
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
                    .owner
                    .as_ref()
                    .is_some_and(|owner| hierarchy.external_members(file, owner).contains(name))
                // A method of a node visitor of `ast`, which its `visit` calls by the name of the node.
                || (definition.kind == Kind::Method
                    && name.starts_with(NODE_VISITOR_PREFIX)
                    && definition.owner.as_ref().is_some_and(|owner| {
                        let ancestors = hierarchy.ancestors(file, owner);
                        NODE_VISITORS.iter().any(|visitor| ancestors.contains(*visitor))
                    }))
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
        let inherits = definition.owner.as_ref().is_some_and(|owner| {
            let ancestors = hierarchy.ancestors(file, owner);
            item.base_classes
                .iter()
                .any(|base| ancestors.contains(base))
        });
        if !inherits {
            return false;
        }
    }
    // An override with class decorators applies to what is defined in a class that has one of them.
    item.class_decorators.is_empty()
        || definition.owner.as_ref().is_some_and(|owner| {
            let decorators = hierarchy.decorators(file, owner);
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
    // By the owner of their code, which is None for no function.
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
        let container = match (&definition.owner, definition.function) {
            (Some(owner), _) => classes.get(&(*file, owner.clone())),
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
        let Some(&file) = file_of.get(path) else {
            continue;
        };
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
    uses: &'a FxHashMap<(usize, Owner), Uses>,
    /// The members of each enum of the project, by its file and its name in the module, with their
    /// names.
    enums: FxHashMap<(usize, String), (Vec<DefinitionId>, FxHashSet<String>)>,
    by_name: FxHashMap<(usize, String), Vec<DefinitionId>>,
    by_attribute: FxHashMap<String, Vec<DefinitionId>>,
    by_key: FxHashMap<String, Vec<DefinitionId>>,
    public: FxHashMap<usize, Vec<DefinitionId>>,
    /// What is used so far, and what is left to follow.
    used: Vec<bool>,
    pending: Vec<(usize, Owner)>,
    seen_owners: FxHashSet<(usize, Owner)>,
    seen: Seen,
}

/// What the uses of the owners followed so far read, so that each is looked up once.
#[derive(Default)]
struct Seen {
    names: FxHashSet<(usize, String)>,
    attributes: FxHashSet<String>,
    keys: FxHashSet<String>,
    objects: FxHashSet<(usize, Vec<String>)>,
    stars: FxHashSet<usize>,
}

impl<'a> Graph<'a> {
    fn new(
        project: &'a Project,
        uses: &'a FxHashMap<(usize, Owner), Uses>,
        fields: &FxHashSet<DefinitionId>,
        enums: FxHashMap<(usize, String), Vec<DefinitionId>>,
    ) -> Self {
        let mut by_name: FxHashMap<(usize, String), Vec<DefinitionId>> = FxHashMap::default();
        let mut by_attribute: FxHashMap<String, Vec<DefinitionId>> = FxHashMap::default();
        let mut by_key: FxHashMap<String, Vec<DefinitionId>> = FxHashMap::default();
        let mut public: FxHashMap<usize, Vec<DefinitionId>> = FxHashMap::default();
        for (id, (file, definition)) in project
            .definition_files
            .iter()
            .zip(&project.definitions)
            .enumerate()
        {
            // A function or class that is a local variable of a function is read by its definition.
            let is_local = project.facts[*file]
                .references
                .local_definitions
                .contains(&(id - project.first_definition[*file]));
            if !is_local {
                by_name
                    .entry((*file, definition.name.clone()))
                    .or_default()
                    .push(id);
            }
            // A field of a TypedDict is read as a key, and only a method or another attribute is read
            // as an attribute of an object.
            if fields.contains(&id) {
                by_key.entry(definition.name.clone()).or_default().push(id);
            } else if definition.owner.is_some() {
                by_attribute
                    .entry(definition.name.clone())
                    .or_default()
                    .push(id);
            }
            // A star import imports the names in `__all__`, which are references of the module
            // already, or every public name of the module if there is no `__all__`. A method, an
            // attribute, or a function or class that is a local variable of a function is not a name
            // of the module.
            if !project.facts[*file].references.has_all
                && !definition.name.starts_with('_')
                && definition.owner.is_none()
                && !is_local
            {
                public.entry(*file).or_default().push(id);
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

        Self {
            project,
            uses,
            enums,
            by_name,
            by_attribute,
            by_key,
            public,
            used: vec![false; project.definitions.len()],
            // The code of every module runs.
            pending: (0..project.files.len()).map(|file| (file, None)).collect(),
            seen_owners: FxHashSet::default(),
            seen: Seen::default(),
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
        while let Some(owner) = self.pending.pop() {
            let Some(uses) = self.uses.get(&owner) else {
                continue;
            };
            if !self.seen_owners.insert(owner) {
                continue;
            }
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
    fn reached(&self, id: DefinitionId) -> FxHashSet<DefinitionId> {
        let (file, definition) = (
            &self.project.definition_files[id],
            &self.project.definitions[id],
        );
        let mut reached = FxHashSet::default();
        if !definition.kind.is_function() {
            return reached;
        }
        let Some(uses) = self.uses.get(&(*file, Some(definition.line))) else {
            return reached;
        };
        self.look_up(uses, None, &mut reached);
        reached
    }

    /// Find the definitions that the uses of an owner can stand for: the names by their module, the
    /// attributes and keys by their name, the members of the enums that are read as a whole, and the
    /// public names of the modules that are star imported, and the functions and classes defined in
    /// a function that the code reads.
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
                        .get(attribute)
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
    fn enums_read(&self, file: usize, parts: &[String]) -> Vec<DefinitionId> {
        let mut found = Vec::new();
        for index in 1..=parts.len() {
            if let Some((members, names)) = self.enums.get(&(file, parts[..index].join(".")))
                && (index == parts.len() || !names.contains(&parts[index]))
            {
                found.extend(members);
            }
        }
        found
    }

    /// Mark definitions as used, and queue the code of the functions among them.
    fn add(&mut self, definitions: impl IntoIterator<Item = DefinitionId>) {
        for id in definitions {
            if !self.used[id] {
                self.used[id] = true;
                let (file, definition) = (
                    &self.project.definition_files[id],
                    &self.project.definitions[id],
                );
                if definition.kind.is_function() {
                    self.pending.push((*file, Some(definition.line)));
                }
            }
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
            .map(|attributes| attributes.as_str().split('.').collect())
            .unwrap_or_default();
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
