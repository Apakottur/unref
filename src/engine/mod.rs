//! The unused definitions of a Python project, which are those that nothing references.

mod class_members;
mod findings;
mod graph;
mod imports;
mod roots;
mod uses;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::classes::{ClassHierarchy, join};
use crate::config::Config;
use crate::definitions::{Definition, Kind};
use crate::discover::discover;
use crate::external::ExternalModules;
use crate::files::{FileFacts, analyze_files};
use crate::frameworks::{FrameworkRules, ProjectFile};
use crate::models::{Finding, Framework};
use crate::modules::{ModuleName, module_name, resolve};
use crate::symbols::ModuleSymbols;

use class_members::Classes;
use findings::{findings, transitive_definitions};
use graph::Graph;
use imports::{
    import_siblings, imported_objects, is_external, is_external_import, resolve_import, search_path,
};
use roots::find_roots;
use uses::{Uses, find_uses};

/// `TypedDict`, as `typing` and `typing_extensions` give it out.
const TYPED_DICTS: [&str; 2] = ["typing.TypedDict", "typing_extensions.TypedDict"];
/// `Enum`, which `IntEnum`, `StrEnum`, `Flag` and the other enums of `enum` inherit from.
const ENUMS: [&str; 1] = ["enum.Enum"];

/// A definition of the project, by its index among the definitions of every file in order.
type DefinitionId = usize;

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
    import_siblings(&mut facts, &names, &files_by_name, &mut external);

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
            Arc::new(ModuleSymbols::of(
                project.files[index].clone(),
                project.names[index].clone(),
                project.file_definitions(index),
                &file_facts.references,
                file_facts.lazy.clone(),
            ))
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
    let uses = find_uses(&project, &classes, &typed_dicts);

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
    let mut graph = Graph::new(&project, &uses, &fields, enums, implementations);

    // An ignore comment is needed for a definition that is unused without it. Such a definition is
    // then used, and so is what it uses.
    let kept_used = graph.use_definitions(kept.iter().copied());
    let needed: Vec<DefinitionId> = ignored.into_iter().filter(|id| !kept_used[*id]).collect();
    let mut all_used = graph.use_definitions(needed.iter().copied());
    share_used(&project, &mut all_used);
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

/// Mark a definition used if it is used in another file of the same source. A link to a file is a
/// module of its own, with its own name, but deleting a definition from it deletes it from both.
fn share_used(project: &Project, used: &mut [bool]) {
    let mut by_source: FxHashMap<PathBuf, Vec<usize>> = FxHashMap::default();
    for (file, path) in project.files.iter().enumerate() {
        by_source.entry(resolve(path)).or_default().push(file);
    }
    for files in by_source.values().filter(|files| files.len() > 1) {
        // The files have the same source, so they have the same definitions in the same order.
        let starts: Vec<DefinitionId> = files
            .iter()
            .map(|&file| project.first_definition[file])
            .collect();
        for offset in 0..project.ids_of_file(files[0]).len() {
            if starts.iter().any(|start| used[start + offset]) {
                for start in &starts {
                    used[start + offset] = true;
                }
            }
        }
    }
}
