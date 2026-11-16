//! The classes of a project, with the base classes that they inherit from and their decorators.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::external::ExternalModules;
use crate::references::Binding;
use crate::symbols::ModuleSymbols;

/// A set of dotted names that is cached and shared.
pub(crate) type Names = Rc<FxHashSet<String>>;

/// The classes of a project, with the base classes that they inherit from and their decorators.
///
/// A base class is followed through the imports of the project, so that a class is known to inherit
/// from the base classes of its base classes, wherever they are defined. A base class that is imported
/// from outside the project is followed through the source of its module, such as `enum.StrEnum` to
/// `enum.Enum`. A decorator is followed through the imports as well, to where it is defined.
///
/// A module is known by its index: the modules of the project come first, by the index of their file,
/// and the modules outside the project that are read are added after them.
pub(crate) struct ClassHierarchy {
    project_count: usize,
    modules: RefCell<Vec<Arc<ModuleSymbols>>>,
    by_path: RefCell<FxHashMap<PathBuf, usize>>,
    files_by_name: FxHashMap<String, Vec<usize>>,
    external: RefCell<ExternalModules>,
    ancestors: Cache,
    inherited: Cache,
    resolved_decorators: Cache,
    resolved: Cache,
}

impl ClassHierarchy {
    /// Make the hierarchy from the symbols of each module of the project, by the index of its file, the
    /// files of the project by their module name, and the modules outside the project.
    pub(crate) fn new(
        modules: Vec<Arc<ModuleSymbols>>,
        files_by_name: FxHashMap<String, Vec<usize>>,
        external: ExternalModules,
    ) -> Self {
        let by_path = modules
            .iter()
            .enumerate()
            .map(|(index, module)| (module.path.to_path_buf(), index))
            .collect();
        Self {
            project_count: modules.len(),
            modules: RefCell::new(modules),
            by_path: RefCell::new(by_path),
            files_by_name,
            external: RefCell::new(external),
            ancestors: RefCell::default(),
            inherited: RefCell::default(),
            resolved_decorators: RefCell::default(),
            resolved: RefCell::default(),
        }
    }

    fn module(&self, index: usize) -> Arc<ModuleSymbols> {
        self.modules.borrow()[index].clone()
    }

    /// Get the base classes that a class inherits from, directly or not, by their dotted names, such as
    /// `pydantic.BaseModel`. A base class that is imported through several modules has the name of each.
    ///
    /// `module` is the module of the class and `name` its name in the module.
    pub(crate) fn ancestors(&self, module: usize, name: &str) -> Names {
        cached(&self.ancestors, module, name, || {
            let mut seen = FxHashSet::default();
            let mut found = FxHashSet::default();
            let symbols = self.module(module);
            for base in symbols.bases.get(name).into_iter().flatten() {
                found.extend(self.resolve_written(module, base, &mut seen));
            }
            found
        })
    }

    /// Get the names of the members that a class inherits from the base classes outside the project.
    ///
    /// Code outside the project can use such a member, such as `logging.Handler` calling `emit`, so a
    /// member of the class with the same name overrides one that is used.
    ///
    /// Returns the names of the methods and attributes that the base classes outside the project
    /// define, in their body or on their instances.
    pub(crate) fn external_members(&self, module: usize, name: &str) -> Names {
        cached(&self.inherited, module, name, || {
            let mut found = FxHashSet::default();
            for ancestor in self.ancestors(module, name).iter() {
                for (file, symbol) in self.locate(ancestor) {
                    // The members of the modules of the project are not collected.
                    if file >= self.project_count
                        && let Some(members) = self.module(file).members.get(&symbol)
                    {
                        found.extend(members.iter().cloned());
                    }
                }
            }
            found
        })
    }

    /// Get the decorators of a class, by their dotted names, such as `dataclasses.dataclass`. A
    /// decorator that is imported through several modules has the name of each.
    pub(crate) fn decorators(&self, module: usize, name: &str) -> Names {
        cached(&self.resolved_decorators, module, name, || {
            let mut found = FxHashSet::default();
            let symbols = self.module(module);
            for decorator in symbols.decorators.get(name).into_iter().flatten() {
                found.extend(self.resolve(module, decorator).iter().cloned());
            }
            found
        })
    }

    /// Get the dotted names of a name as it is written in a module, such as the decorator of a
    /// function: `pytest.fixture` for `fixture` after `from pytest import fixture`. A name that is
    /// imported through several modules has the name of each.
    pub(crate) fn resolve(&self, module: usize, written: &str) -> Names {
        cached(&self.resolved, module, written, || {
            self.resolve_written(module, written, &mut FxHashSet::default())
        })
    }

    /// Get the dotted names of a base class or decorator, as it is written in a module, and of the
    /// base classes of a base class.
    fn resolve_written(
        &self,
        module: usize,
        written: &str,
        seen: &mut FxHashSet<String>,
    ) -> FxHashSet<String> {
        let symbols = self.module(module);
        let (first, rest) = written.split_once('.').unwrap_or((written, ""));

        if symbols.bases.contains_key(first) {
            // A class or function of the module itself.
            return self.follow(&format!("{}.{written}", symbols.name.name), seen);
        }

        if let Some(binding) = symbols.bindings.get(first) {
            // A name that the module imports.
            return match imported(&symbols, binding) {
                None => FxHashSet::default(),
                Some(imported) => self.follow(&join(&imported, rest), seen),
            };
        }

        // A builtin, or a name that the module does not define at its top level.
        FxHashSet::from_iter([written.to_string()])
    }

    /// Get a dotted name, and the dotted names of what it stands for in the project: the base classes
    /// of a class, and the origin of an imported name.
    fn follow(&self, name: &str, seen: &mut FxHashSet<String>) -> FxHashSet<String> {
        if !seen.insert(name.to_string()) {
            return FxHashSet::default();
        }

        let mut found = FxHashSet::from_iter([name.to_string()]);
        for (file, symbol) in self.locate(name) {
            let symbols = self.module(file);
            let (first, rest) = symbol.split_once('.').unwrap_or((&symbol, ""));
            if let Some(bases) = symbols.bases.get(&symbol) {
                for base in bases {
                    found.extend(self.resolve_written(file, base, seen));
                }
            } else if let Some(binding) = symbols.bindings.get(first) {
                if let Some(imported) = imported(&symbols, binding) {
                    found.extend(self.follow(&join(&imported, rest), seen));
                }
            } else if !symbols.bases.contains_key(first) {
                // A name that the module may star import. A star import of a submodule binds the
                // submodule itself under its name, so that name is the submodule and not one that a
                // star import gives out.
                let star_modules: FxHashSet<Option<String>> = symbols
                    .stars
                    .iter()
                    .map(|(source, level)| symbols.name.resolve(source.as_deref(), *level))
                    .collect();
                if !star_modules.contains(&Some(format!("{}.{first}", symbols.name.name))) {
                    for star_module in star_modules.into_iter().flatten() {
                        found.extend(self.follow(&format!("{star_module}.{symbol}"), seen));
                    }
                }
                // A name that the `__getattr__` of the module imports, which it gives out.
                for binding in symbols.lazy.get(first).into_iter().flatten() {
                    if let Some(imported) = imported(&symbols, binding) {
                        found.extend(self.follow(&join(&imported, rest), seen));
                    }
                }
            }
        }
        found
    }

    /// Get the modules that a dotted name is in, each with the rest of the name in the module, such as
    /// `Outer.Inner` for a nested class.
    ///
    /// The longest start of the name that is a module of the project is taken, and otherwise the
    /// longest one that is a module outside the project. A module outside the project without a source,
    /// such as a compiled module, has no file, and a shorter start is not taken in its place.
    fn locate(&self, name: &str) -> Vec<(usize, String)> {
        let parts: Vec<&str> = name.split('.').collect();
        let starts: Vec<(String, String)> = (1..parts.len())
            .rev()
            .map(|index| (parts[..index].join("."), parts[index..].join(".")))
            .collect();
        for (module, rest) in &starts {
            if let Some(files) = self.files_by_name.get(module) {
                return files.iter().map(|&file| (file, rest.clone())).collect();
            }
        }
        for (module, rest) in starts {
            if self.external.borrow_mut().exists(&module) {
                return self
                    .read_external(&module)
                    .into_iter()
                    .map(|file| (file, rest.clone()))
                    .collect();
            }
        }
        Vec::new()
    }

    /// Get the module that is imported from outside the project by its name, and add its symbols to
    /// those of the project.
    fn read_external(&self, name: &str) -> Option<usize> {
        let external = self.external.borrow_mut().get(name)?;
        let path = external.path.to_path_buf();
        if let Some(&index) = self.by_path.borrow().get(&path) {
            return Some(index);
        }
        let mut modules = self.modules.borrow_mut();
        modules.push(external);
        let index = modules.len() - 1;
        self.by_path.borrow_mut().insert(path, index);
        Some(index)
    }
}

/// A cache of sets of dotted names, by a module and a name in it.
type Cache = RefCell<FxHashMap<(usize, String), Names>>;

/// Get the names cached for a name of a module, or compute them and cache them.
///
/// The cache is not borrowed while the names are computed, since computing them can read this cache
/// and the others.
fn cached(
    cache: &Cache,
    module: usize,
    name: &str,
    compute: impl FnOnce() -> FxHashSet<String>,
) -> Names {
    let key = (module, name.to_string());
    if let Some(found) = cache.borrow().get(&key) {
        return found.clone();
    }
    let found = Rc::new(compute());
    cache.borrow_mut().insert(key, found.clone());
    found
}

/// Get the dotted name that an imported name of a module stands for.
fn imported(module: &ModuleSymbols, binding: &Binding) -> Option<String> {
    let source = module
        .name
        .resolve(binding.module.as_deref(), binding.level)?;
    Some(match &binding.name {
        Some(name) => join(&source, name),
        None => source,
    })
}

/// Join two dotted names, either of which can be empty.
fn join(first: &str, second: &str) -> String {
    match (first.is_empty(), second.is_empty()) {
        (_, true) => first.to_string(),
        (true, false) => second.to_string(),
        (false, false) => format!("{first}.{second}"),
    }
}
