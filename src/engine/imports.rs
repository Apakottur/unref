//! Whether a name is from outside the project, where the modules outside it are looked for, and what
//! the imports of the scripts of a package stand for.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ruff_python_stdlib::builtins::is_python_builtin;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::external::{ExternalModules, environment_paths, standard_library};
use crate::files::FileFacts;
use crate::modules::{ModuleName, import_root, resolve};
use crate::references::Binding;
use crate::resolver::{Resolver, imported_name, join};

use super::Project;

/// Check whether a function, as it is written in a module, is from outside the project: a builtin, or
/// a name that the module imports from outside the project. Any other name, such as a method of a local
/// variable, is not known.
pub(super) fn is_external(
    project: &Project,
    resolver: &Resolver,
    file: usize,
    written: &str,
) -> bool {
    let root = written.split('.').next().unwrap_or(written);
    // The builtins of any version of Python.
    let is_known = project.facts[file].references.bindings.contains_key(root)
        || is_python_builtin(root, u8::MAX, false);
    is_known
        && resolver
            .aliases_written(file, written)
            .iter()
            .all(|name| !project.holds(name))
}

/// Check whether a function, as it is written after a function-local import of its first name, is
/// from outside the project.
pub(super) fn is_external_import(
    project: &Project,
    file: usize,
    binding: &Binding,
    written: &str,
) -> bool {
    let attributes = written
        .split_once('.')
        .map_or("", |(_, attributes)| attributes);
    imported_name(&project.names[file], binding)
        .is_some_and(|imported| !project.holds(&join(&imported, attributes)))
}

/// Get the directories that a module from outside the project is looked for in.
///
/// A module from outside the project is looked for where the modules of the project are imported from
/// first, as an excluded module is, then in the directories of `PYTHONPATH`, in the site-packages of
/// the environment of the project, if there is one, and then in the standard library.
pub(super) fn search_path(
    files: &[Arc<Path>],
    names: &[ModuleName],
    python: Option<&Path>,
) -> Vec<PathBuf> {
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
pub(super) fn import_siblings(
    facts: &mut [FileFacts],
    names: &[ModuleName],
    files_by_name: &FxHashMap<String, Vec<usize>>,
    external: &mut ExternalModules,
) {
    // The names of the modules and packages of the project, with every package above a module.
    let known: FxHashSet<&str> = files_by_name
        .keys()
        .flat_map(|name| {
            name.match_indices('.')
                .map(|(index, _)| &name[..index])
                .chain([name.as_str()])
        })
        .collect();
    let package_of = |file: usize| {
        let name = &names[file];
        (!name.is_package)
            .then(|| name.name.rsplit_once('.').map(|(package, _)| package))
            .flatten()
    };
    let mut pending: Vec<usize> = (0..facts.len())
        .filter(|&file| facts[file].is_script && package_of(file).is_some())
        .collect();
    let mut seen: FxHashSet<usize> = pending.iter().copied().collect();
    let mut is_elsewhere: FxHashMap<String, bool> = FxHashMap::default();
    while let Some(file) = pending.pop() {
        let Some(package) = package_of(file) else {
            continue;
        };
        let mut siblings: Vec<String> = Vec::new();
        import_relative(&mut facts[file], |module, level| {
            let Some(top) = module.and_then(|module| module.split('.').next()) else {
                return false;
            };
            let sibling = format!("{package}.{top}");
            let is_sibling = level == 0
                && !known.contains(top)
                && known.contains(sibling.as_str())
                && !*is_elsewhere
                    .entry(top.to_string())
                    .or_insert_with(|| external.exists(top));
            if is_sibling {
                siblings.push(sibling);
            }
            is_sibling
        });
        for sibling in siblings {
            for &imported in files_by_name.get(&sibling).into_iter().flatten() {
                if package_of(imported).is_some() && seen.insert(imported) {
                    pending.push(imported);
                }
            }
        }
    }
}

/// Make the imports of a file that `is_sibling` tells, by their module and their number of leading
/// dots, relative imports from its package.
fn import_relative(
    file_facts: &mut FileFacts,
    mut is_sibling: impl FnMut(Option<&str>, u32) -> bool,
) {
    let references = &mut file_facts.references;
    references.imports = std::mem::take(&mut references.imports)
        .into_iter()
        .map(|mut import| {
            if is_sibling(import.module.as_deref(), import.level) {
                import.level = 1;
            }
            import
        })
        .collect();
    references.imported_reads = std::mem::take(&mut references.imported_reads)
        .into_iter()
        .map(|mut read| {
            if is_sibling(read.binding.module.as_deref(), read.binding.level) {
                read.binding.level = 1;
            }
            read
        })
        .collect();
    for binding in references
        .bindings
        .values_mut()
        .chain(file_facts.lazy.values_mut().flatten())
    {
        if is_sibling(binding.module.as_deref(), binding.level) {
            binding.level = 1;
        }
    }
}
