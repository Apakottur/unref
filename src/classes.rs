//! The classes of a project, with the base classes that they inherit from and their decorators.

use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::LOCALS;
use crate::resolver::{Names, Resolver, Trace};

/// The classes of a project, with the base classes that they inherit from and their decorators, on top
/// of the name resolver of the project.
///
/// A base class is followed through the imports of the project, so that a class is known to inherit
/// from the base classes of its base classes, wherever they are defined. A base class that is imported
/// from outside the project is followed through the source of its module, such as `enum.StrEnum` to
/// `enum.Enum`. A decorator is followed through the imports as well, to where it is defined.
pub(crate) struct ClassHierarchy {
    resolver: Resolver,
    ancestors: Cache,
    inherited: Cache,
    resolved_decorators: Cache,
    resolved: Cache,
    followed: RefCell<FxHashMap<String, Names>>,
}

impl ClassHierarchy {
    /// Make the hierarchy on top of the name resolver of the project.
    pub(crate) fn new(resolver: Resolver) -> Self {
        Self {
            resolver,
            ancestors: RefCell::default(),
            inherited: RefCell::default(),
            resolved_decorators: RefCell::default(),
            resolved: RefCell::default(),
            followed: RefCell::default(),
        }
    }

    /// The name resolver of the project, which the hierarchy follows names with.
    pub(crate) fn resolver(&self) -> &Resolver {
        &self.resolver
    }

    /// Get the base classes that a class inherits from, directly or not, by their dotted names, such as
    /// `pydantic.BaseModel`. A base class that is imported through several modules has the name of each.
    ///
    /// `module` is the module of the class and `name` its name in the module.
    pub(crate) fn ancestors(&self, module: usize, name: &str) -> Names {
        cached(&self.ancestors, module, name, || {
            let mut found = FxHashSet::default();
            let symbols = self.resolver.module(module);
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
                for (file, symbol) in self.resolver.locate(ancestor) {
                    // The members of the modules of the project are not collected.
                    if !self.resolver.is_project(file)
                        && let Some(members) = self.resolver.module(file).members.get(&symbol)
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
            let symbols = self.resolver.module(module);
            for decorator in symbols.decorators.get(name).into_iter().flatten() {
                found.extend(self.resolve_in(module, name, decorator).iter().cloned());
            }
            found
        })
    }

    /// Get the dotted names of a name as it is written in a module, such as the decorator of a
    /// function: `pytest.fixture` for `fixture` after `from pytest import fixture`, with the base
    /// classes of a class, directly or not. A name that is imported through several modules has the
    /// name of each.
    ///
    /// Where in the module the name is written is not known, so a name that the module neither
    /// defines at its top level nor imports is each class or function of that name in its functions,
    /// and each import of it in them.
    pub(crate) fn resolve(&self, module: usize, written: &str) -> Names {
        cached(&self.resolved, module, written, || {
            let mut trace = Trace::default();
            self.resolver.trace_written(module, "", written, &mut trace);
            self.with_ancestors(trace)
        })
    }

    /// Get the dotted names of a name as it is written where a class or function of a module is
    /// defined, such as a base class of the class. `scope` is the name of that class or function in
    /// the module: one that is defined in a function sees the classes and functions of that function.
    pub(crate) fn resolve_in(&self, module: usize, scope: &str, written: &str) -> Names {
        if !scope.contains(LOCALS) {
            return self.resolve(module, written);
        }
        let mut trace = Trace::default();
        self.resolver
            .trace_written(module, scope, written, &mut trace);
        Rc::new(self.with_ancestors(trace))
    }

    /// Get the dotted names that a dotted name stands for: the name itself, the origin of an imported
    /// name, and the base classes of a class, directly or not.
    pub(crate) fn follow_name(&self, name: &str) -> Names {
        if let Some(found) = self.followed.borrow().get(name) {
            return found.clone();
        }
        let mut trace = Trace::default();
        self.resolver.trace(name.to_string(), &mut trace);
        let found = Rc::new(self.with_ancestors(trace));
        self.followed
            .borrow_mut()
            .insert(name.to_string(), found.clone());
        found
    }

    /// Follow the base classes of the classes that a trace reaches, and of theirs in turn, and get the
    /// dotted names of the trace with those of the base classes.
    fn with_ancestors(&self, mut trace: Trace) -> FxHashSet<String> {
        while let Some((file, class)) = trace.definitions.pop() {
            let symbols = self.resolver.module(file);
            for base in symbols.bases.get(&class).into_iter().flatten() {
                self.resolver.trace_written(file, &class, base, &mut trace);
            }
        }
        trace.names
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
