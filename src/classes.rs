//! The classes of a project, with the base classes that they inherit from and their decorators.

use std::cell::RefCell;
use std::collections::VecDeque;
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
    followed: RefCell<FxHashMap<String, Names>>,
    project_bases: RefCell<FxHashMap<String, Names>>,
    star_imported: RefCell<FxHashMap<String, Rc<StarImport>>>,
    located: RefCell<FxHashMap<String, Located>>,
}

/// A name that a module may star import, followed through the star imports of the modules.
struct StarImport {
    /// The names on the way to the modules that give the name out, with the names in those modules.
    names: FxHashSet<String>,
    /// The names in the modules that give the name out, which are followed further from there.
    sources: Vec<String>,
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
            followed: RefCell::default(),
            project_bases: RefCell::default(),
            star_imported: RefCell::default(),
            located: RefCell::default(),
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
                self.resolve_written(module, base, &mut seen, &mut found, false);
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
            let mut found = FxHashSet::default();
            self.resolve_written(
                module,
                written,
                &mut FxHashSet::default(),
                &mut found,
                false,
            );
            found
        })
    }

    /// Follow a name whose import was resolved in the scope where it was read.
    pub(crate) fn resolve_binding(&self, module: usize, binding: &Binding, rest: &str) -> Names {
        imported(&self.module(module), binding)
            .map_or_else(Names::default, |name| self.follow_name(&join(&name, rest)))
    }

    /// Follow an imported base through re-exports and external bases, stopping at project classes.
    /// Their bases are tracked by definition in the project.
    pub(crate) fn imported_project_bases(
        &self,
        module: usize,
        binding: &Binding,
        rest: &str,
    ) -> Names {
        let Some(name) = imported(&self.module(module), binding) else {
            return Names::default();
        };
        let name = join(&name, rest);
        if let Some(found) = self.project_bases.borrow().get(&name) {
            return found.clone();
        }
        let mut found = FxHashSet::default();
        self.follow(&name, &mut FxHashSet::default(), &mut found, true);
        let found = Rc::new(found);
        self.project_bases.borrow_mut().insert(name, found.clone());
        found
    }

    /// Get the dotted names that a dotted name stands for: the name itself, the origin of an imported
    /// name, and the base classes of a class, directly or not.
    pub(crate) fn follow_name(&self, name: &str) -> Names {
        if let Some(found) = self.followed.borrow().get(name) {
            return found.clone();
        }
        let mut found = FxHashSet::default();
        self.follow(name, &mut FxHashSet::default(), &mut found, false);
        let found = Rc::new(found);
        self.followed
            .borrow_mut()
            .insert(name.to_string(), found.clone());
        found
    }

    /// Add the dotted names of a base class or decorator, as it is written in a module, and of the
    /// base classes of a base class.
    fn resolve_written(
        &self,
        module: usize,
        written: &str,
        seen: &mut FxHashSet<String>,
        found: &mut FxHashSet<String>,
        stop_at_project: bool,
    ) {
        let symbols = self.module(module);
        let (first, rest) = written.split_once('.').unwrap_or((written, ""));

        if symbols.bases.contains_key(first) {
            // A class or function of the module itself.
            self.follow(
                &format!("{}.{written}", symbols.name.name),
                seen,
                found,
                stop_at_project,
            );
        } else if let Some(binding) = symbols.bindings.get(first) {
            // A name that the module imports.
            if let Some(imported) = imported(&symbols, binding) {
                self.follow(&join(&imported, rest), seen, found, stop_at_project);
            }
        } else {
            // A builtin, or a name that the module does not define at its top level.
            found.insert(written.to_string());
        }
    }

    /// Add a dotted name, and the dotted names of what it stands for in the project: the base classes
    /// of a class, and the origin of an imported name.
    ///
    /// The names are added to one set as they are found, rather than returned and copied at each step
    /// of a long chain of imports.
    fn follow(
        &self,
        name: &str,
        seen: &mut FxHashSet<String>,
        found: &mut FxHashSet<String>,
        stop_at_project: bool,
    ) {
        if !seen.insert(name.to_string()) {
            return;
        }

        found.insert(name.to_string());
        for (file, symbol) in self.locate(name) {
            let symbols = self.module(file);
            let (first, rest) = symbol.split_once('.').unwrap_or((&symbol, ""));
            let project = stop_at_project && file < self.project_count;
            if project && symbols.exported_classes.contains(first) {
                continue;
            }
            if !project && let Some(bases) = symbols.bases.get(&symbol) {
                for base in bases {
                    self.resolve_written(file, base, seen, found, stop_at_project);
                }
            } else if let Some(binding) = symbols.bindings.get(first) {
                if let Some(imported) = imported(&symbols, binding) {
                    self.follow(&join(&imported, rest), seen, found, stop_at_project);
                }
            } else if project || !symbols.bases.contains_key(first) {
                // A name that the module may star import.
                for star_module in star_modules(&symbols, first) {
                    let star = self.star_imported(&join(star_module, &symbol));
                    found.extend(star.names.iter().cloned());
                    for source in &star.sources {
                        self.follow(source, seen, found, stop_at_project);
                    }
                }
                // A name that the `__getattr__` of the module imports, which it gives out.
                for binding in symbols.lazy.get(first).into_iter().flatten() {
                    if let Some(imported) = imported(&symbols, binding) {
                        self.follow(&join(&imported, rest), seen, found, stop_at_project);
                    }
                }
            }
        }
    }

    /// Follow a name through the star imports of the modules, to the modules that give it out.
    ///
    /// `name` is a name in a module that the module may star import, such as `package.Model` when
    /// `package` star imports `package.models`, which defines `Model`. A module gives out a name that
    /// it defines, imports or gives out from its `__getattr__`. A module that is not found, such as a
    /// compiled one, may give out any name.
    ///
    /// The star imports of a package often import the package back, so they are followed through
    /// their cycles once for each name, rather than once for each class that reads the name.
    fn star_imported(&self, name: &str) -> Rc<StarImport> {
        if let Some(found) = self.star_imported.borrow().get(name) {
            return found.clone();
        }

        // The name that each name is first star imported by, which is on the shortest way to it. The
        // other ways go round a cycle, through names that are no more the name than the shortest.
        let mut importers: FxHashMap<String, Option<String>> = FxHashMap::default();
        importers.insert(name.to_string(), None);
        let mut pending = VecDeque::from([name.to_string()]);
        let mut sources = Vec::new();
        while let Some(current) = pending.pop_front() {
            let located = self.locate(&current);
            if located.is_empty() {
                sources.push(current);
                continue;
            }
            let mut gives_out = false;
            for (file, symbol) in located {
                let symbols = self.module(file);
                let first = symbol
                    .split_once('.')
                    .map_or(symbol.as_str(), |(first, _)| first);
                if symbols.bases.contains_key(&symbol)
                    || symbols.bindings.contains_key(first)
                    || symbols.lazy.contains_key(first)
                {
                    gives_out = true;
                } else if !symbols.bases.contains_key(first) {
                    for star_module in star_modules(&symbols, first) {
                        // A star import of a module that is not found gives out no names. Its name
                        // would otherwise be looked for in the package above it, which can star import
                        // it again, with a longer name each time.
                        if !self.exists(star_module) {
                            continue;
                        }
                        let imported = join(star_module, &symbol);
                        if !importers.contains_key(&imported) {
                            importers.insert(imported.clone(), Some(current.clone()));
                            pending.push_back(imported);
                        }
                    }
                }
            }
            if gives_out {
                sources.push(current);
            }
        }

        // The names on the shortest way to each module that gives the name out. A name that leads to
        // none is not a name of it.
        let mut names: FxHashSet<String> = FxHashSet::default();
        for source in &sources {
            let mut current = Some(source);
            while let Some(name) = current
                && names.insert(name.clone())
            {
                current = importers[name].as_ref();
            }
        }

        let found = Rc::new(StarImport { names, sources });
        self.star_imported
            .borrow_mut()
            .insert(name.to_string(), found.clone());
        found
    }

    /// Get the modules that a dotted name is in, each with the rest of the name in the module, such as
    /// `Outer.Inner` for a nested class.
    ///
    /// The longest start of the name that is a module of the project is taken, and otherwise the
    /// longest one that is a module outside the project. A module outside the project without a source,
    /// such as a compiled module, has no file, and a shorter start is not taken in its place.
    fn locate(&self, name: &str) -> Vec<(usize, String)> {
        let Some((module, last)) = name.rsplit_once('.') else {
            return Vec::new();
        };
        let cached = self.located.borrow().get(module).cloned();
        let located = cached.unwrap_or_else(|| {
            let located: Located = self.locate_module(module).into();
            self.located
                .borrow_mut()
                .insert(module.to_string(), located.clone());
            located
        });
        located
            .iter()
            .map(|(file, rest)| (*file, join(rest, last)))
            .collect()
    }

    /// Get the modules that the longest start of a dotted name is, as `locate` takes them, each with
    /// the rest of the name.
    fn locate_module(&self, name: &str) -> Vec<(usize, String)> {
        let parts: Vec<&str> = name.split('.').collect();
        let starts: Vec<(String, String)> = (1..=parts.len())
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

    /// Tell whether a module is found, in the project or outside it.
    fn exists(&self, name: &str) -> bool {
        self.files_by_name.contains_key(name) || self.external.borrow_mut().exists(name)
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

/// The modules that a dotted name is in, each with the rest of the name in the module.
type Located = Rc<[(usize, String)]>;

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

/// Get the modules that a module star imports, to look up a name in them by its first part.
///
/// A star import of a submodule binds the submodule itself under its name, so that name is the
/// submodule and not one that a star import gives out.
fn star_modules<'a>(module: &'a ModuleSymbols, first: &str) -> &'a [String] {
    if module
        .stars
        .binary_search(&format!("{}.{first}", module.name.name))
        .is_ok()
    {
        return &[];
    }
    &module.stars
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
pub(crate) fn join(first: &str, second: &str) -> String {
    match (first.is_empty(), second.is_empty()) {
        (_, true) => first.to_string(),
        (true, false) => second.to_string(),
        (false, false) => format!("{first}.{second}"),
    }
}
