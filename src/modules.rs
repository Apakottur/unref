//! The names that Python files are imported by.

use std::path::{Component, Path, PathBuf};

const INIT: &str = "__init__.py";

/// The name that a Python file is imported by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleName {
    /// The dotted name, such as `package.module`. The `__init__.py` of a package has the name of the
    /// package.
    pub name: String,
    /// Whether the file is the `__init__.py` of a package.
    pub is_package: bool,
}

impl ModuleName {
    /// Get the dotted name of the module that an import in this module imports from.
    ///
    /// `module` is the module of the import as it is written: `helpers` in
    /// `from .helpers import name`, and None in `from . import name`. `level` is the number of
    /// leading dots of the import, 0 for an absolute import.
    ///
    /// Returns None if the import has more dots than there are packages.
    pub(crate) fn resolve(&self, module: Option<&str>, level: u32) -> Option<String> {
        if level == 0 {
            return module.map(ToString::to_string);
        }

        // One dot is the package of this module, and each further dot is one package up.
        let mut package: Vec<&str> = if self.name.is_empty() {
            Vec::new()
        } else {
            self.name.split('.').collect()
        };
        if !self.is_package {
            package.pop();
        }
        let level = usize::try_from(level).unwrap_or(usize::MAX);
        if level > package.len() {
            return None;
        }
        package.truncate(package.len() - level + 1);
        if let Some(module) = module {
            package.push(module);
        }
        Some(package.join("."))
    }
}

/// Get the name that a Python file is imported by.
///
/// The name is made of the file and of the directories above it that are packages. A directory is a
/// package if it has an `__init__.py`, or if it is a namespace package: a directory without one below
/// a package, both under one of the given directories. The first directory that is not a package is
/// taken to be on the import path.
///
/// `roots` are the directories given to the check, resolved. A namespace package is only looked for
/// under them, so that a package outside the project does not pull directories in.
pub(crate) fn module_name(path: &Path, roots: &[PathBuf]) -> ModuleName {
    let path = resolve(path);
    let is_package = path.file_name().is_some_and(|name| name == INIT);
    let mut parts: Vec<String> = Vec::new();
    if !is_package {
        parts.push(stem(&path));
    }

    let mut directory = path.parent();
    while let Some(current) = directory {
        if !(current.join(INIT).is_file() || is_namespace_package(current, roots)) {
            break;
        }
        let Some(name) = current.file_name() else {
            break;
        };
        parts.push(name.to_string_lossy().into_owned());
        directory = current.parent();
    }
    parts.reverse();

    ModuleName {
        name: parts.join("."),
        is_package,
    }
}

/// Check whether a directory without an `__init__.py` is a namespace package of the project, which is
/// to say that it is below a package with one, both under one of the given directories.
fn is_namespace_package(directory: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| {
        directory.starts_with(root)
            && directory
                .ancestors()
                .skip(1)
                .take_while(|ancestor| ancestor.starts_with(root))
                .any(|ancestor| ancestor.join(INIT).is_file())
    })
}

/// Get the directory on the import path that a Python file is imported from: the directory that holds
/// the first package of the name, or the directory of the file if it is not in a package.
pub(crate) fn import_root(path: &Path, name: &ModuleName) -> PathBuf {
    let depth = name.name.matches('.').count() + usize::from(name.is_package);
    let path = resolve(path);
    path.ancestors()
        .nth(depth + 1)
        .unwrap_or(Path::new("/"))
        .to_path_buf()
}

/// The name of a file without its last suffix.
fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Make a path absolute and resolve its links, as Python's `Path.resolve` does, also for a path that
/// does not exist.
pub(crate) fn resolve(path: &Path) -> PathBuf {
    if let Ok(resolved) = path.canonicalize() {
        return resolved;
    }
    let mut absolute = PathBuf::new();
    for component in std::path::absolute(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .components()
    {
        match component {
            // A `..` goes up from where the path before it leads, after its links.
            Component::ParentDir => {
                absolute = absolute.canonicalize().unwrap_or(absolute);
                absolute.pop();
            }
            Component::CurDir => {}
            other => absolute.push(other),
        }
    }
    if let Ok(resolved) = absolute.canonicalize() {
        return resolved;
    }
    // The longest start of the path that exists is resolved, and the rest is kept as it is.
    let mut rest = Vec::new();
    let mut current = absolute.as_path();
    while let Some(parent) = current.parent() {
        if let Some(name) = current.file_name() {
            rest.push(name.to_os_string());
        }
        if let Ok(resolved) = parent.canonicalize() {
            let mut result = resolved;
            for name in rest.iter().rev() {
                result.push(name);
            }
            return result;
        }
        current = parent;
    }
    absolute
}

#[cfg(test)]
mod tests {
    use super::ModuleName;

    #[test]
    fn resolve() {
        let module = ModuleName {
            name: "package.sub.module".to_string(),
            is_package: false,
        };
        assert_eq!(module.resolve(Some("os"), 0).as_deref(), Some("os"));
        assert_eq!(
            module.resolve(Some("helpers"), 1).as_deref(),
            Some("package.sub.helpers")
        );
        assert_eq!(module.resolve(None, 1).as_deref(), Some("package.sub"));
        assert_eq!(module.resolve(None, 2).as_deref(), Some("package"));
        assert_eq!(module.resolve(None, 3), None);

        let package = ModuleName {
            name: "package".to_string(),
            is_package: true,
        };
        assert_eq!(
            package.resolve(Some("module"), 1).as_deref(),
            Some("package.module")
        );
        assert_eq!(package.resolve(None, 2), None);
    }
}
