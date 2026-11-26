//! What a name stands for, as it is written in a module of a project, through the imports of the
//! modules of the project and of the modules outside it.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::LOCALS;
use crate::external::ExternalModules;
use crate::modules::ModuleName;
use crate::references::Binding;
use crate::symbols::ModuleSymbols;

/// The most parts of a dotted name that is followed, far more than a name in real code has.
const MAX_PARTS: usize = 32;

/// A set of dotted names that is cached and shared.
pub(crate) type Names = Rc<FxHashSet<String>>;

/// The name resolver of a project: what a name, as it is written in a module, stands for.
///
/// A name is followed through the imports of the modules at their top level, through their star
/// imports, and through the names that a module gives out from its `__getattr__`. A name that is
/// imported from outside the project is followed through the source of its module, such as
/// `enum.StrEnum` to `enum.Enum`. A name that is imported through several modules has the name of
/// each, such as `pydantic.main.BaseModel` and `pydantic.BaseModel`, so that a name given by the user
/// matches any of them.
///
/// A module is known by its index: the modules of the project come first, by the index of their file,
/// and the modules outside the project that are read are added after them.
pub(crate) struct Resolver {
    project_count: usize,
    modules: RefCell<Vec<Arc<ModuleSymbols>>>,
    by_path: RefCell<FxHashMap<PathBuf, usize>>,
    files_by_name: FxHashMap<String, Vec<usize>>,
    external: RefCell<ExternalModules>,
    project_aliases: RefCell<FxHashMap<String, Names>>,
    star_imported: RefCell<FxHashMap<String, Rc<StarImport>>>,
    located: RefCell<FxHashMap<String, Located>>,
}

/// What names stand for, as they are followed: the dotted names on the way, and the classes and
/// functions that they reach.
///
/// A name that is reached twice is followed once, which ends a cycle of modules that import the names
/// of each other.
#[derive(Default)]
pub(crate) struct Trace {
    seen: FxHashSet<String>,
    /// Whether the names are only followed through the modules of the project.
    within_project: bool,
    /// The dotted names that the names stand for, the names themselves among them.
    pub(crate) names: FxHashSet<String>,
    /// The classes and functions that the names reach, each with its module and its name in the
    /// module, in the order that they are reached.
    pub(crate) definitions: Vec<(usize, String)>,
}

/// A name that a module may star import, followed through the star imports of the modules.
struct StarImport {
    /// The names on the way to the modules that give the name out, with the names in those modules.
    names: FxHashSet<String>,
    /// The names in the modules that give the name out, which are followed further from there.
    sources: Vec<String>,
}

/// The modules that a dotted name is in, each with the rest of the name in the module.
type Located = Rc<[(usize, String)]>;

impl Resolver {
    /// Make the resolver from the symbols of each module of the project, by the index of its file, the
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
            project_aliases: RefCell::default(),
            star_imported: RefCell::default(),
            located: RefCell::default(),
        }
    }

    /// Get the symbols of a module, of the project or outside it, by its index.
    pub(crate) fn module(&self, index: usize) -> Arc<ModuleSymbols> {
        self.modules.borrow()[index].clone()
    }

    /// Tell whether a module, by its index, is a module of the project.
    pub(crate) fn is_project(&self, index: usize) -> bool {
        index < self.project_count
    }

    /// Get the dotted names that a dotted name stands for through the modules of the project: the name
    /// itself and the origins of an imported name. A class or function is not followed further, nor is
    /// a name that no module of the project holds, such as `os.path.join`, as reading the sources of
    /// the modules outside the project for each such name takes time.
    pub(crate) fn project_aliases(&self, name: &str) -> Names {
        if let Some(found) = self.project_aliases.borrow().get(name) {
            return found.clone();
        }
        let mut trace = Trace {
            within_project: true,
            ..Trace::default()
        };
        self.trace(name.to_string(), &mut trace);
        let found = Rc::new(trace.names);
        self.project_aliases
            .borrow_mut()
            .insert(name.to_string(), found.clone());
        found
    }

    /// Get the dotted names that a name, as it is written in a module, stands for, such as
    /// `pytest.fixture` for `fixture` after `from pytest import fixture`. A class or function is not
    /// followed further.
    pub(crate) fn aliases_written(&self, module: usize, written: &str) -> FxHashSet<String> {
        let mut trace = Trace::default();
        self.trace_written(module, "", written, &mut trace);
        trace.names
    }

    /// Follow a name as it is written in a module, and add what it stands for to a trace.
    ///
    /// `scope` is the name in the module of the class or function that it is written for, or an empty
    /// string at the top level of the module. The classes, functions and imports of the functions that
    /// it is defined in are looked in first, from the nearest one. Where in the module the name is
    /// written is not known otherwise, so a name that the module neither defines at its top level nor
    /// imports is each class or function of that name in its functions, and each import of it in them.
    pub(crate) fn trace_written(
        &self,
        module: usize,
        scope: &str,
        written: &str,
        trace: &mut Trace,
    ) {
        let symbols = self.module(module);
        let (first, rest) = written.split_once('.').unwrap_or((written, ""));

        let mut functions = scope;
        while let Some((function, _)) = functions.rsplit_once(LOCALS) {
            let local = format!("{function}{LOCALS}{first}");
            if symbols.bases.contains_key(&local) {
                // A class or function of a function that the scope is in.
                self.trace(join(&symbols.name.name, &join(&local, rest)), trace);
                return;
            }
            if let Some(binding) = symbols
                .local_bindings
                .get(function)
                .and_then(|bindings| bindings.get(first))
            {
                // A name that a function that the scope is in imports.
                self.trace_binding(&symbols.name, binding, rest, trace);
                return;
            }
            functions = function;
        }
        if symbols.bases.contains_key(first) || symbols.aliases.contains_key(first) {
            // A class or function of the module itself, or a variable of it that is assigned one.
            self.trace(format!("{}.{written}", symbols.name.name), trace);
        } else if let Some(binding) = symbols.bindings.get(first) {
            // A name that the module imports.
            self.trace_binding(&symbols.name, binding, rest, trace);
        } else if scope.contains(LOCALS) {
            // A builtin, or a name that the module does not define at its top level.
            trace.names.insert(written.to_string());
        } else {
            // A name that is not written in a function of the module, or that is written where it is
            // not known, is each class or function of that name that a function of the module defines,
            // and each module or name that a function of the module imports by that name.
            let locals = symbols.locals.get(first).into_iter().flatten();
            let imported = symbols
                .local_bindings
                .values()
                .filter_map(|bindings| bindings.get(first));
            let mut found = false;
            for local in locals {
                self.trace(join(&symbols.name.name, &join(local, rest)), trace);
                found = true;
            }
            for binding in imported {
                self.trace_binding(&symbols.name, binding, rest, trace);
                found = true;
            }
            if !found {
                // A builtin, or a name that the module does not define.
                trace.names.insert(written.to_string());
            }
        }
    }

    /// Follow what an imported name stands for, with the attributes read from it, and add it to a
    /// trace.
    fn trace_binding(&self, module: &ModuleName, binding: &Binding, rest: &str, trace: &mut Trace) {
        if let Some(imported) = imported_name(module, binding) {
            self.trace(join(&imported, rest), trace);
        }
    }

    /// Follow a dotted name, and add it and the dotted names that it stands for to a trace, through
    /// the imports, the star imports and the `__getattr__` of the modules that it is in, up to the
    /// classes and functions that it reaches.
    pub(crate) fn trace(&self, name: String, trace: &mut Trace) {
        let mut pending = vec![name];
        while let Some(name) = pending.pop() {
            // A package that imports a name from a submodule it does not have, as `from first.first
            // import first` in `first`, makes the name longer at each step instead of repeating it.
            if name.split('.').nth(MAX_PARTS).is_some() || !trace.seen.insert(name.clone()) {
                continue;
            }
            // A name that no module of the project holds is not looked for in the sources of the
            // modules outside it.
            if trace.within_project && !self.holds(&name) {
                trace.names.insert(name);
                continue;
            }
            for (file, symbol) in self.locate(&name) {
                let symbols = self.module(file);
                // A module that is found by its name from the project root also stands for its name,
                // which the rest of the project knows it by.
                let named = join(&symbols.name.name, &symbol);
                if named != name {
                    pending.push(named);
                }
                let (first, rest) = symbol.split_once('.').unwrap_or((&symbol, ""));
                // A variable that is assigned a dotted name stands for each name that it is assigned,
                // as in both branches of an `if`, besides a class or import of that name.
                for value in symbols.aliases.get(first).into_iter().flatten() {
                    self.trace_written(file, "", &join(value, rest), trace);
                }
                if symbols.bases.contains_key(&symbol) {
                    trace.definitions.push((file, symbol));
                } else if let Some(binding) = symbols.bindings.get(first) {
                    pending.extend(
                        imported_name(&symbols.name, binding).map(|imported| join(&imported, rest)),
                    );
                } else if !symbols.bases.contains_key(first) && !symbols.aliases.contains_key(first)
                {
                    // A name that the module may star import.
                    for star_module in star_modules(&symbols, first) {
                        let star = self.star_imported(&join(star_module, &symbol));
                        trace.names.extend(star.names.iter().cloned());
                        pending.extend(star.sources.iter().cloned());
                    }
                    // A name that the `__getattr__` of the module imports, which it gives out.
                    for binding in symbols.lazy.get(first).into_iter().flatten() {
                        pending.extend(
                            imported_name(&symbols.name, binding)
                                .map(|imported| join(&imported, rest)),
                        );
                    }
                }
            }
            trace.names.insert(name);
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
    pub(crate) fn locate(&self, name: &str) -> Vec<(usize, String)> {
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

    /// Tell whether a dotted name is in a module of the project: whether a start of it, shorter than
    /// the whole, is the name of one.
    fn holds(&self, name: &str) -> bool {
        name.match_indices('.')
            .any(|(index, _)| self.files_by_name.contains_key(&name[..index]))
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

/// Get the dotted name that an imported name of a module stands for, such as `package.module.name`
/// for `from .module import name` in `package`.
///
/// Returns None if it is imported from beyond the root.
pub(crate) fn imported_name(module: &ModuleName, binding: &Binding) -> Option<String> {
    let source = module.resolve(binding.module.as_deref(), binding.level)?;
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
