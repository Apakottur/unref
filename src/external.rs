//! The modules that are imported from outside the project, and the environments they are found in.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::files::analyze_file;
use crate::modules::{ModuleName, resolve};
use crate::symbols::ModuleSymbols;

/// The suffixes of compiled modules, besides those with a tag such as `.cpython-313-x86_64-linux-gnu.so`.
const EXTENSION_SUFFIXES: [&str; 2] = ["so", "pyd"];

/// Where a module is found: its file, if it has one, and whether it is a package.
type Located = Option<(Option<PathBuf>, bool)>;

/// The modules that are imported from outside the project.
///
/// A module is found on a search path the way Python finds it, and read from its source, but it is
/// never imported: its code does not run. A module without a Python source, such as a builtin or a
/// compiled module, is not read.
pub(crate) struct ExternalModules {
    /// The directories that modules are imported from, in order.
    search_path: Vec<PathBuf>,
    modules: FxHashMap<String, Option<Arc<ModuleSymbols>>>,
    found: FxHashMap<String, Located>,
    /// The entries of each directory that was looked in, each with whether it is a directory.
    listings: FxHashMap<PathBuf, FxHashMap<OsString, bool>>,
}

impl ExternalModules {
    pub(crate) fn new(search_path: Vec<PathBuf>) -> Self {
        Self {
            search_path,
            modules: FxHashMap::default(),
            found: FxHashMap::default(),
            listings: FxHashMap::default(),
        }
    }

    /// Tell whether a module is found, whether or not it has a source that can be read, such as a
    /// compiled module or a namespace package.
    pub(crate) fn exists(&mut self, name: &str) -> bool {
        self.locate(name).is_some()
    }

    /// Get a module by its dotted name, or None if it is not found or does not parse.
    pub(crate) fn get(&mut self, name: &str) -> Option<Arc<ModuleSymbols>> {
        if let Some(module) = self.modules.get(name) {
            return module.clone();
        }
        let module = self.read(name).map(Arc::new);
        self.modules.insert(name.to_string(), module.clone());
        module
    }

    /// Find a module, with its file and whether it is a package.
    fn locate(&mut self, name: &str) -> Located {
        if let Some(found) = self.found.get(name) {
            return found.clone();
        }
        // Each package of the name is looked for in turn, in the directories of the package above it.
        let parts: Vec<&str> = name.split('.').collect();
        let mut locations: Option<Vec<PathBuf>> = Some(self.search_path.clone());
        let mut result = None;
        for (index, part) in parts.iter().enumerate() {
            // A module that is not a package has no submodules.
            let Some(directories) = locations.take() else {
                break;
            };
            let Some((file, found_locations)) = self.find(part, &directories) else {
                break;
            };
            locations = found_locations;
            if index == parts.len() - 1 {
                result = Some((file, locations.is_some()));
            }
        }
        self.found.insert(name.to_string(), result.clone());
        result
    }

    /// Find a module and read its source.
    fn read(&mut self, name: &str) -> Option<ModuleSymbols> {
        // A module that is not found has no file, as a namespace package.
        let (file, is_package) = self.locate(name)?;
        let file = file?;
        if file.extension().is_none_or(|extension| extension != "py") {
            return None;
        }
        let file: Arc<Path> = Arc::from(file);
        let facts = analyze_file(&file).ok()?;
        Some(ModuleSymbols::of(
            file,
            ModuleName {
                name: name.to_string(),
                is_package,
            },
            &facts.definitions,
            &facts.references,
            facts.lazy,
        ))
    }

    /// The entries of a directory, each with whether it is a directory, following links.
    fn listing(&mut self, directory: &Path) -> &FxHashMap<OsString, bool> {
        self.listings
            .entry(directory.to_path_buf())
            .or_insert_with(|| {
                let Ok(entries) = std::fs::read_dir(directory) else {
                    return FxHashMap::default();
                };
                entries
                    .filter_map(Result::ok)
                    .map(|entry| {
                        let is_directory = match entry.file_type() {
                            Ok(kind) if kind.is_symlink() => entry.path().is_dir(),
                            Ok(kind) => kind.is_dir(),
                            Err(_) => false,
                        };
                        (entry.file_name(), is_directory)
                    })
                    .collect()
            })
    }

    /// Find the file of a module in a directory, in the order that Python looks for it: a compiled
    /// module, a source, and a bytecode file.
    fn module_file(&mut self, directory: &Path, stem: &str) -> Option<PathBuf> {
        let listing = self.listing(directory);
        let is_file = |name: &str| listing.get(&OsString::from(name)) == Some(&false);
        let compiled = listing.iter().find_map(|(name, &is_directory)| {
            let name = name.to_str()?;
            let rest = name.strip_prefix(stem)?.strip_prefix('.')?;
            let compiled = !is_directory
                && EXTENSION_SUFFIXES.iter().any(|suffix| {
                    rest == *suffix
                        || rest
                            .strip_suffix(suffix)
                            .and_then(|tag| tag.strip_suffix('.'))
                            .is_some_and(|tag| !tag.is_empty() && !tag.contains('.'))
                });
            compiled.then(|| name.to_string())
        });
        let name = compiled
            .or_else(|| is_file(&format!("{stem}.py")).then(|| format!("{stem}.py")))
            .or_else(|| is_file(&format!("{stem}.pyc")).then(|| format!("{stem}.pyc")))?;
        Some(directory.join(name))
    }

    /// Find a module in the directories that it can be imported from, in the order that Python looks
    /// for it.
    ///
    /// `name` is the name of the module in its package, without dots, and `locations` the directories
    /// of the search path, or of the package of the module.
    ///
    /// Returns the file of the module, and the directories of its submodules if it is a package. A
    /// namespace package, which is a directory without an `__init__.py`, has no file, and is made of
    /// the directories of its name in every location. None if the module is not found.
    fn find(
        &mut self,
        name: &str,
        locations: &[PathBuf],
    ) -> Option<(Option<PathBuf>, Option<Vec<PathBuf>>)> {
        let mut portions = Vec::new();
        for location in locations {
            let is_directory = self.listing(location).get(&OsString::from(name)) == Some(&true);
            if is_directory {
                let directory = location.join(name);
                if let Some(init) = self.module_file(&directory, "__init__") {
                    return Some((Some(init), Some(vec![directory])));
                }
                portions.push(directory);
            }
            if let Some(file) = self.module_file(location, name) {
                return Some((Some(file), None));
            }
        }
        if portions.is_empty() {
            None
        } else {
            Some((None, Some(portions)))
        }
    }
}

/// Get the directories that a Python environment imports its installed modules from, without running
/// its interpreter.
///
/// They are its `site-packages`, followed by the directories that the `.pth` files there add, in the
/// order that Python's `site` module adds them. A line of a `.pth` file that imports code is not run.
///
/// `environment` is given as ty takes it: a virtual environment such as `.venv`, a `sys.prefix` such
/// as `/usr`, or an interpreter such as `.venv/bin/python`.
///
/// Returns the directories that exist, in order.
pub(crate) fn environment_paths(environment: &Path) -> Vec<PathBuf> {
    let environment = environment_directory(environment);
    let mut site_packages_directories = Vec::new();
    // `lib/python3.X/site-packages` on POSIX, and `Lib/site-packages` on Windows.
    for lib in sorted_entries(&environment, |name| name.starts_with("lib")) {
        for version in sorted_entries(&lib, |name| name.starts_with("python3")) {
            let site_packages = version.join("site-packages");
            if site_packages.is_dir() {
                site_packages_directories.push(site_packages);
            }
        }
    }
    let windows = environment.join("Lib").join("site-packages");
    if windows.is_dir() {
        site_packages_directories.push(windows);
    }
    site_packages_directories.sort();
    site_packages_directories.dedup();

    let mut paths = Vec::new();
    for site_packages in site_packages_directories {
        paths.push(site_packages.clone());
        for pth in sorted_entries(&site_packages, |name| {
            Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "pth")
        }) {
            let Ok(bytes) = std::fs::read(&pth) else {
                continue;
            };
            let text = String::from_utf8_lossy(&bytes);
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
            for line in text.lines() {
                if line.trim().is_empty()
                    || line.starts_with('#')
                    || line.starts_with("import ")
                    || line.starts_with("import\t")
                {
                    continue;
                }
                let directory = site_packages.join(line.trim_end());
                if directory.is_dir() {
                    paths.push(directory);
                }
            }
        }
    }
    // `lib64` is often a link to `lib`.
    dedup_resolved(paths)
}

/// Get the directories of the standard library of the Python that a project runs on.
///
/// The Python of the environment of the project is used when there is one: the one that a virtual
/// environment is made from, which its `pyvenv.cfg` names, or the environment itself if it is a
/// `sys.prefix`. Otherwise it is the first `python3` or `python` on the `PATH`.
pub(crate) fn standard_library(environment: Option<&Path>) -> Vec<PathBuf> {
    let (prefixes, version) = match environment {
        Some(environment) => {
            let environment = environment_directory(environment);
            match read_pyvenv(&environment) {
                Some((home, version)) => {
                    let prefixes = home
                        .parent()
                        .map(Path::to_path_buf)
                        .into_iter()
                        .chain([home])
                        .collect();
                    (prefixes, version)
                }
                None => (vec![environment], None),
            }
        }
        None => (path_python_prefixes(), None),
    };
    for prefix in prefixes {
        let found = standard_library_of(&prefix, version.as_deref());
        if !found.is_empty() {
            return dedup_resolved(found);
        }
    }
    Vec::new()
}

/// Find the standard library under a `sys.prefix`, preferring a given version such as `3.15`.
fn standard_library_of(prefix: &Path, version: Option<&str>) -> Vec<PathBuf> {
    // `Lib` on Windows.
    let windows = prefix.join("Lib");
    if windows.join("os.py").is_file() {
        return vec![windows, prefix.join("DLLs")]
            .into_iter()
            .filter(|path| path.is_dir())
            .collect();
    }
    let lib = prefix.join("lib");
    let mut candidates: Vec<PathBuf> = sorted_entries(&lib, |name| name.starts_with("python3"))
        .into_iter()
        .filter(|directory| directory.join("os.py").is_file())
        .collect();
    // The newest version, unless one is asked for.
    candidates.sort_by_key(|directory| {
        let name = directory
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let minor: u32 = name
            .trim_start_matches("python3.")
            .trim_end_matches('t')
            .parse()
            .unwrap_or(0);
        (
            version.is_some_and(|version| name == format!("python{version}")),
            minor,
        )
    });
    candidates.pop().map_or_default(|directory| {
        vec![directory.clone(), directory.join("lib-dynload")]
            .into_iter()
            .filter(|path| path.is_dir())
            .collect()
    })
}

/// Read the `home` and the version of Python, such as `3.15`, of a virtual environment.
fn read_pyvenv(environment: &Path) -> Option<(PathBuf, Option<String>)> {
    let text = std::fs::read_to_string(environment.join("pyvenv.cfg")).ok()?;
    let mut home = None;
    let mut version = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "home" => home = Some(PathBuf::from(value)),
            "version" | "version_info" => {
                let mut parts = value.split('.');
                if let (Some(major), Some(minor)) = (parts.next(), parts.next()) {
                    version = Some(format!("{major}.{minor}"));
                }
            }
            _ => {}
        }
    }
    Some((home?, version))
}

/// The prefixes of the Pythons on the `PATH`, in the order they are found.
fn path_python_prefixes() -> Vec<PathBuf> {
    let Some(path) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let names: &[&str] = if cfg!(windows) {
        &["python3.exe", "python.exe"]
    } else {
        &["python3", "python"]
    };
    let mut prefixes = Vec::new();
    for directory in std::env::split_paths(&path) {
        for name in names {
            let Ok(interpreter) = directory.join(name).canonicalize() else {
                continue;
            };
            let Some(parent) = interpreter.parent() else {
                continue;
            };
            // `bin/python3` on POSIX, and `python.exe` in the prefix itself on Windows.
            let prefix = if parent.file_name().is_some_and(|name| name == "bin") {
                parent.parent().unwrap_or(parent)
            } else {
                parent
            };
            prefixes.push(prefix.to_path_buf());
        }
    }
    prefixes
}

/// The directory of an environment, given by the directory or by its interpreter.
fn environment_directory(environment: &Path) -> PathBuf {
    if !environment.is_file() {
        return environment.to_path_buf();
    }
    interpreter_environment(environment)
}

/// The environment of an interpreter, or of a command installed with it.
///
/// An interpreter is in the `bin` or `Scripts` directory of its environment, or, for a system Python
/// on Windows, directly in it. Its link is not followed: the interpreter of a virtual environment
/// links to the one it was created from.
pub(crate) fn interpreter_environment(interpreter: &Path) -> PathBuf {
    let parent = interpreter.parent().unwrap_or(Path::new(""));
    if parent
        .file_name()
        .is_some_and(|name| name == "bin" || name == "Scripts")
    {
        parent.parent().unwrap_or(parent).to_path_buf()
    } else {
        parent.to_path_buf()
    }
}

/// The entries of a directory whose name matches, sorted.
fn sorted_entries(directory: &Path, matches: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().is_some_and(&matches))
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found
}

/// Resolve paths, and keep the first of each.
fn dedup_resolved(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = rustc_hash::FxHashSet::default();
    paths
        .into_iter()
        .map(|path| resolve(&path))
        .filter(|path| seen.insert(path.clone()))
        .collect()
}
