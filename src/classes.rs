//! The classes of a project, with the base classes that they inherit from and their decorators.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::LOCALS;
use crate::external::ExternalModules;
use crate::references::Binding;
use crate::symbols::ModuleSymbols;

/// A set of dotted names that is cached and shared.
pub(crate) type Names = Rc<FxHashSet<String>>;
/// The absolute imports whose source depends on the script directory, by importing file and name.
pub(crate) type ScriptImports = FxHashMap<usize, FxHashMap<String, Vec<(usize, String)>>>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ImportContext {
    /// All possible sources, for declarations analyzed without a caller.
    Any,
    Ordinary,
    Script(usize),
}

#[derive(Default)]
pub(crate) struct Followed {
    pub(crate) names: FxHashSet<String>,
    pub(crate) depends_on_script: bool,
}

struct Follow {
    context: ImportContext,
    seen: FxHashSet<String>,
    active_bindings: FxHashMap<(usize, *const Binding), usize>,
    result: Followed,
}

impl Follow {
    fn new(context: ImportContext) -> Self {
        Self {
            context,
            seen: FxHashSet::default(),
            active_bindings: FxHashMap::default(),
            result: Followed::default(),
        }
    }
}

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
    script_imports: ScriptImports,
    followed: ImportCache<Followed>,
    star_imported: ImportCache<StarImport>,
    located: RefCell<FxHashMap<String, Located>>,
}

/// A name that a module may star import, followed through the star imports of the modules.
struct StarImport {
    /// The names on the way to the modules that give the name out, with the names in those modules.
    names: FxHashSet<String>,
    /// The names in the modules that give the name out, which are followed further from there.
    sources: Vec<String>,
    depends_on_script: bool,
}

impl ClassHierarchy {
    /// Make the hierarchy from the symbols of each module of the project, by the index of its file, the
    /// files of the project by their module name, and the modules outside the project.
    pub(crate) fn new(
        modules: Vec<Arc<ModuleSymbols>>,
        files_by_name: FxHashMap<String, Vec<usize>>,
        external: ExternalModules,
        script_imports: ScriptImports,
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
            script_imports,
            followed: RefCell::default(),
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
            let mut found = FxHashSet::default();
            let symbols = self.module(module);
            for base in symbols.bases.get(name).into_iter().flatten() {
                found.extend(self.resolve_in(module, name, base).iter().cloned());
            }
            found
        })
    }

    /// Check whether a class inherits from one of the given classes, by their dotted names, directly
    /// or not.
    pub(crate) fn inherits(&self, module: usize, name: &str, bases: &[impl AsRef<str>]) -> bool {
        let ancestors = self.ancestors(module, name);
        bases.iter().any(|base| ancestors.contains(base.as_ref()))
    }

    /// Check whether a name, as it is written in a module, resolves to one of the given dotted names.
    pub(crate) fn resolves_to(&self, module: usize, written: &str, names: &[&str]) -> bool {
        let resolved = self.resolve(module, written);
        names.iter().any(|name| resolved.contains(*name))
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
                found.extend(self.resolve_in(module, name, decorator).iter().cloned());
            }
            found
        })
    }

    /// Get the dotted names of a name as it is written in a module, such as the decorator of a
    /// function: `pytest.fixture` for `fixture` after `from pytest import fixture`. A name that is
    /// imported through several modules has the name of each.
    ///
    /// Where in the module the name is written is not known, so a name that the module neither
    /// defines at its top level nor imports is each class or function of that name in its functions.
    pub(crate) fn resolve(&self, module: usize, written: &str) -> Names {
        cached(&self.resolved, module, written, || {
            let mut walk = Follow::new(ImportContext::Any);
            self.resolve_written(module, "", written, &mut walk);
            walk.result.names
        })
    }

    /// Get the dotted names of a name as it is written where a class or function of a module is
    /// defined, such as a base class of the class. `scope` is the name of that class or function in
    /// the module: one that is defined in a function sees the classes and functions of that function.
    pub(crate) fn resolve_in(&self, module: usize, scope: &str, written: &str) -> Names {
        if !scope.contains(LOCALS) {
            return self.resolve(module, written);
        }
        let mut walk = Follow::new(ImportContext::Any);
        self.resolve_written(module, scope, written, &mut walk);
        Rc::new(walk.result.names)
    }

    /// Follow an imported name and the base classes, allowing the sources for the running script.
    pub(crate) fn follow_name(&self, name: &str, scope: Option<usize>) -> Rc<Followed> {
        let ordinary = self.follow_name_in_context(name, ImportContext::Ordinary);
        if ordinary.depends_on_script
            && let Some(scope) = scope
        {
            return self.follow_name_in_context(name, ImportContext::Script(scope));
        }
        ordinary
    }

    pub(crate) fn has_script_imports(&self) -> bool {
        !self.script_imports.is_empty()
    }

    /// A class may have a different name when re-exported through an import or `__getattr__`.
    pub(crate) fn can_reexport(&self, name: &str) -> bool {
        self.has_script_imports()
            && name.rsplit_once('.').is_some_and(|(module, symbol)| {
                self.files_by_name.get(module).is_some_and(|files| {
                    files
                        .iter()
                        .filter(|file| self.script_imports.contains_key(file))
                        .any(|&file| {
                            let symbols = self.module(file);
                            symbols.bindings.contains_key(symbol)
                                || symbols.lazy.contains_key(symbol)
                                || !symbols.stars.is_empty()
                        })
                })
            })
    }

    fn follow_name_in_context(&self, name: &str, context: ImportContext) -> Rc<Followed> {
        if let Some(found) = import_cached(&self.followed, context, name) {
            return found;
        }
        let mut walk = Follow::new(context);
        self.follow(name, &mut walk);
        let found = Rc::new(walk.result);
        self.followed
            .borrow_mut()
            .entry(context)
            .or_default()
            .insert(name.to_string(), found.clone());
        found
    }

    /// Add the dotted names of a base class or decorator, as it is written in a module, and of the
    /// base classes of a base class.
    ///
    /// `scope` is the name in the module of the class or function that it is written for, or an empty
    /// string at the top level of the module. The functions that it is defined in are looked in first,
    /// from the nearest one.
    fn resolve_written(&self, module: usize, scope: &str, written: &str, walk: &mut Follow) {
        let symbols = self.module(module);
        let (first, rest) = written.split_once('.').unwrap_or((written, ""));

        let mut functions = scope;
        while let Some((function, _)) = functions.rsplit_once(LOCALS) {
            let local = format!("{function}{LOCALS}{first}");
            if symbols.bases.contains_key(&local) {
                // A class or function of a function that the scope is in.
                let name = join(&symbols.name.name, &join(&local, rest));
                self.follow(&name, walk);
                return;
            }
            functions = function;
        }
        if symbols.bases.contains_key(first) {
            // A class or function of the module itself.
            self.follow(&format!("{}.{written}", symbols.name.name), walk);
        } else if let Some(bindings) = symbols.bindings.get(first) {
            // A name that the module imports.
            for binding in bindings {
                self.follow_binding(module, &symbols, binding, rest, walk);
            }
        } else if !scope.contains(LOCALS)
            && let Some(locals) = symbols.locals.get(first)
        {
            // A name that is not written in a function of the module, or that is written where it is
            // not known, is each class or function of that name that a function of the module defines.
            for local in locals {
                let name = join(&symbols.name.name, &join(local, rest));
                self.follow(&name, walk);
            }
        } else {
            // A builtin, or a name that the module does not define at its top level.
            walk.result.names.insert(written.to_string());
        }
    }

    /// Add a dotted name, and the dotted names of what it stands for in the project: the base classes
    /// of a class, and the origin of an imported name.
    ///
    /// The names are added to one set as they are found, rather than returned and copied at each step
    /// of a long chain of imports.
    fn follow(&self, name: &str, walk: &mut Follow) {
        if !walk.seen.insert(name.to_string()) {
            return;
        }

        walk.result.names.insert(name.to_string());
        for (file, symbol) in self.locate(name) {
            let symbols = self.module(file);
            let (first, rest) = symbol.split_once('.').unwrap_or((&symbol, ""));
            if let Some(bases) = symbols.bases.get(&symbol) {
                for base in bases {
                    self.resolve_written(file, &symbol, base, walk);
                }
            } else if let Some(bindings) = symbols.bindings.get(first) {
                for binding in bindings {
                    self.follow_binding(file, &symbols, binding, rest, walk);
                }
            } else if !symbols.bases.contains_key(first) {
                // A name that the module may star import.
                for star_module in self.star_modules(
                    file,
                    &symbols,
                    first,
                    walk.context,
                    &mut walk.result.depends_on_script,
                ) {
                    let star = self.star_imported(&join(star_module, &symbol), walk.context);
                    walk.result.depends_on_script |= star.depends_on_script;
                    walk.result.names.extend(star.names.iter().cloned());
                    for source in &star.sources {
                        self.follow(source, walk);
                    }
                }
                // A name that the `__getattr__` of the module imports, which it gives out.
                for binding in symbols.lazy.get(first).into_iter().flatten() {
                    self.follow_binding(file, &symbols, binding, rest, walk);
                }
            }
        }
    }

    fn follow_binding(
        &self,
        file: usize,
        symbols: &ModuleSymbols,
        binding: &Binding,
        rest: &str,
        walk: &mut Follow,
    ) {
        let Some(source) = symbols
            .name
            .resolve(binding.module.as_deref(), binding.level)
        else {
            return;
        };
        // Each pass through an alias can prepend another namespace to the name, so seeing
        // exact names is insufficient to detect a cycle. A revisit can still be useful
        // when it consumes part of the remaining attribute chain.
        let key = (file, std::ptr::from_ref(binding));
        if walk
            .active_bindings
            .get(&key)
            .is_some_and(|&length| length <= rest.len())
        {
            return;
        }
        let previous = walk.active_bindings.insert(key, rest.len());
        for source in self.import_modules(
            file,
            &source,
            walk.context,
            &mut walk.result.depends_on_script,
        ) {
            let name = join(source, binding.name.as_deref().unwrap_or_default());
            self.follow(&join(&name, rest), walk);
        }
        if let Some(previous) = previous {
            walk.active_bindings.insert(key, previous);
        } else {
            walk.active_bindings.remove(&key);
        }
    }

    fn import_modules<'a>(
        &'a self,
        file: usize,
        source: &'a str,
        context: ImportContext,
        depends_on_script: &mut bool,
    ) -> impl Iterator<Item = &'a str> + use<'a> {
        let alternatives = self
            .script_imports
            .get(&file)
            .and_then(|modules| modules.get(source));
        *depends_on_script |= alternatives.is_some();
        alternatives.is_none().then_some(source).into_iter().chain(
            alternatives
                .into_iter()
                .flatten()
                .filter_map(move |(scope, module)| {
                    (context == ImportContext::Any || context == ImportContext::Script(*scope))
                        .then_some(module.as_str())
                }),
        )
    }

    /// A star import binds the submodule itself under its name. That name comes from the
    /// submodule rather than from a star import.
    fn star_modules<'a>(
        &'a self,
        file: usize,
        symbols: &'a ModuleSymbols,
        first: &str,
        context: ImportContext,
        depends_on_script: &mut bool,
    ) -> Vec<&'a str> {
        let mut modules = Vec::new();
        for star in &symbols.stars {
            modules.extend(self.import_modules(file, star, context, depends_on_script));
        }
        if !modules.is_empty() && modules.contains(&join(&symbols.name.name, first).as_str()) {
            modules.clear();
        }
        modules
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
    fn star_imported(&self, name: &str, context: ImportContext) -> Rc<StarImport> {
        if let Some(found) = import_cached(&self.star_imported, context, name) {
            return found;
        }

        // The name that each name is first star imported by, which is on the shortest way to it. The
        // other ways go round a cycle, through names that are no more the name than the shortest.
        let mut importers: FxHashMap<String, Option<String>> = FxHashMap::default();
        importers.insert(name.to_string(), None);
        let mut pending = VecDeque::from([name.to_string()]);
        let mut sources = Vec::new();
        let mut depends_on_script = false;
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
                    for star_module in
                        self.star_modules(file, &symbols, first, context, &mut depends_on_script)
                    {
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

        let found = Rc::new(StarImport {
            names,
            sources,
            depends_on_script,
        });
        self.star_imported
            .borrow_mut()
            .entry(context)
            .or_default()
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

type ImportCache<T> = RefCell<FxHashMap<ImportContext, FxHashMap<String, Rc<T>>>>;

fn import_cached<T>(cache: &ImportCache<T>, context: ImportContext, name: &str) -> Option<Rc<T>> {
    cache.borrow().get(&context)?.get(name).cloned()
}

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

/// Join two dotted names, either of which can be empty.
pub(crate) fn join(first: &str, second: &str) -> String {
    match (first.is_empty(), second.is_empty()) {
        (_, true) => first.to_string(),
        (true, false) => second.to_string(),
        (false, false) => format!("{first}.{second}"),
    }
}
