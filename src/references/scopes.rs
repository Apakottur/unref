//! The scopes of a module while it is walked: the names that each binds, and where its reads go when
//! it ends.

use ruff_python_ast::Expr;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Binding, ImportedRead, Index, ModulePattern, Read, Receiver, Walker, WholeRead};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeKind {
    Module,
    Class,
    Function,
}

/// The module, a class body, a function, a lambda or a comprehension, while it is walked.
#[derive(Debug)]
pub(super) struct Scope {
    /// Whether the definitions in the scope are module variables, class attributes, or local variables
    /// that are left out. A lambda and a comprehension hold no definitions.
    pub(super) kind: ScopeKind,
    /// The names that the scope binds, which are its local variables.
    pub(super) bound: FxHashSet<String>,
    /// The names that the scope declares `global`.
    pub(super) global_names: FxHashSet<String>,
    /// The names that the scope declares `nonlocal`.
    pub(super) nonlocal_names: FxHashSet<String>,
    /// The names that the scope imports, each with what it stands for.
    pub(super) imports: FxHashMap<String, Binding>,
    /// In a function, the functions and classes that it defines, by their name, each with its index
    /// in the definitions of the module. A scope nested in it passes up those that it defines by a
    /// name that is not local to it, such as one declared `nonlocal`.
    pub(super) functions_and_classes: FxHashMap<String, Vec<usize>>,
    /// The reads of names in the scope, or in a scope nested in it, that are not local to the scopes
    /// in between.
    pub(super) reads: Vec<Read>,
    /// The scope containing a comprehension, when it is not a class body. Assignment expressions
    /// follow this link to bind outside the comprehension.
    pub(super) enclosing: Option<usize>,
    /// The scope that holds the variables of a module, or the attributes of a class, that were
    /// already found. A method shares the names of its class, so that an attribute is found once.
    pub(super) defined_in: Option<usize>,
    /// The variables or attributes that were already found, when the scope holds them, each with its
    /// index in the definitions of the module.
    pub(super) defined: FxHashMap<String, usize>,
    /// In a method, the name of its first parameter, usually `self` or `cls`.
    pub(super) instance: Option<String>,
    /// In a class or a method, the class.
    pub(super) class: Option<Index>,
    /// In a function or a method, the function.
    pub(super) function: Option<Index>,
    /// In a class body, the attributes, methods and classes that it defines so far, by their name,
    /// each with its index in the definitions of the module.
    pub(super) members: FxHashMap<String, Vec<usize>>,
    /// The variables assigned the result of a call, with the dotted names of what they call as
    /// written.
    pub(super) calls: FxHashMap<String, Vec<String>>,
    /// In a function, the parameters and local variables that are annotated, with the dotted names of
    /// the annotation as written, one for each side of a union.
    pub(super) annotations: FxHashMap<String, Vec<String>>,
    /// In a class body, its attributes and those of its instance that are annotated or assigned a
    /// value of a known type, with the dotted names of the types as written.
    pub(super) attribute_types: FxHashMap<String, Vec<String>>,
    /// In a class body, the attributes of its instance that its methods read as a whole, with the
    /// read, whose value is each of the types of the attribute once the class is walked.
    pub(super) attribute_reads: Vec<(String, WholeRead)>,
    /// In a function, the parameters and local variables that are annotated with a container, with the
    /// dotted names of the types of its elements as written.
    pub(super) element_annotations: FxHashMap<String, Vec<String>>,
    /// The names that are assigned a module imported by a string, which are read as imported names.
    pub(super) string_imports: FxHashSet<String>,
    /// The names that are assigned some module whose name matches a pattern, with the pattern.
    pub(super) module_patterns: FxHashMap<String, ModulePattern>,
    /// The scope a comprehension reads from when a name is not local to it.
    pub(super) comprehension_parent: Option<usize>,
    /// The scope a function or lambda reads from when a name is not local to it.
    pub(super) function_parent: Option<usize>,
    /// In a class body, the nearest scope around it that is not a class body. The functions,
    /// lambdas and comprehensions in a class body do not see its names, and neither does a class
    /// body nested in it, so their reads go on to this scope, as do the reads of the class body that
    /// are not of its own names.
    pub(super) outer: Option<usize>,
}

impl Scope {
    pub(super) fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            bound: FxHashSet::default(),
            global_names: FxHashSet::default(),
            nonlocal_names: FxHashSet::default(),
            imports: FxHashMap::default(),
            functions_and_classes: FxHashMap::default(),
            reads: Vec::new(),
            enclosing: None,
            defined_in: None,
            defined: FxHashMap::default(),
            instance: None,
            class: None,
            function: None,
            members: FxHashMap::default(),
            calls: FxHashMap::default(),
            annotations: FxHashMap::default(),
            attribute_types: FxHashMap::default(),
            attribute_reads: Vec::new(),
            element_annotations: FxHashMap::default(),
            string_imports: FxHashSet::default(),
            module_patterns: FxHashMap::default(),
            comprehension_parent: None,
            function_parent: None,
            outer: None,
        }
    }
}

/// Where the reads of a scope that ends go.
#[derive(Clone, Copy)]
pub(super) enum Target {
    /// The enclosing scope.
    Scope(usize),
    /// The reads of the names of the module itself.
    Module,
}

impl Walker<'_, '_> {
    pub(super) fn push_scope(&mut self, scope: Scope) -> usize {
        self.scopes.push(scope);
        self.scopes.len() - 1
    }

    /// Get the scope that the reads of a function, lambda or comprehension that is in a scope go to:
    /// the scope itself, or for a class body, the nearest scope around it that is not a class body.
    pub(super) fn reads_target(&self, scope: usize) -> usize {
        self.scopes[scope].outer.unwrap_or(scope)
    }

    /// Get what a name stands for if a function, lambda or comprehension imports it, the one that the
    /// walker is in or one around it, and no scope in between binds it otherwise.
    pub(super) fn local_import(&self, name: &str) -> Option<&Binding> {
        let mut scope = self.scope;
        while scope != 0 {
            let found = &self.scopes[scope];
            if let Some(binding) = found.imports.get(name) {
                return Some(binding);
            }
            if found.bound.contains(name) {
                return None;
            }
            scope = found.comprehension_parent.or(found.function_parent)?;
        }
        None
    }

    /// Get the dotted names of what a variable is assigned the result of calling, as they are
    /// written, looking through enclosing scopes for a comprehension.
    pub(super) fn variable_calls(&self, name: &str) -> Vec<String> {
        let mut scope = Some(self.scope);
        while let Some(index) = scope {
            let found = &self.scopes[index];
            if found.bound.contains(name) {
                return found.calls.get(name).cloned().unwrap_or_default();
            }
            scope = found.comprehension_parent;
        }
        Vec::new()
    }

    /// Get the dotted names of the annotation of a parameter or local variable, as they are written,
    /// looking through enclosing scopes for a comprehension.
    pub(super) fn variable_annotations(&self, name: &str) -> Vec<String> {
        self.find_annotation(name, |scope| &scope.annotations)
    }

    /// Get the dotted names of the element types of the container annotation of a parameter or local
    /// variable, as they are written, looking through enclosing scopes for a comprehension.
    pub(super) fn variable_element_annotations(&self, name: &str) -> Vec<String> {
        self.find_annotation(name, |scope| &scope.element_annotations)
    }

    /// Get what is kept of the annotation of a parameter or local variable in one of the maps of a
    /// scope, looking through enclosing scopes for a comprehension.
    fn find_annotation(
        &self,
        name: &str,
        annotations: impl Fn(&Scope) -> &FxHashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let mut scope = Some(self.scope);
        while let Some(index) = scope {
            let found = &self.scopes[index];
            // A parameter is not bound in the scope of the body, but its annotation is kept there.
            if found.annotations.contains_key(name) {
                return annotations(found).get(name).cloned().unwrap_or_default();
            }
            if found.bound.contains(name) {
                return Vec::new();
            }
            scope = found.comprehension_parent;
        }
        Vec::new()
    }

    /// Get the class scope and the name of an attribute of the instance, if a value is one, as
    /// `self.options` in a method, looking through enclosing scopes for a comprehension.
    pub(super) fn instance_attribute(&self, value: &Expr) -> Option<(usize, String)> {
        let Expr::Attribute(attribute) = value else {
            return None;
        };
        let Expr::Name(name) = &*attribute.value else {
            return None;
        };
        let mut scope = self.scope;
        loop {
            let found = &self.scopes[scope];
            if found.bound.contains(name.id.as_str()) {
                return None;
            }
            if found.instance.as_deref() == Some(name.id.as_str()) {
                let class = found.defined_in?;
                return (self.scopes[class].kind == ScopeKind::Class)
                    .then(|| (class, attribute.attr.to_string()));
            }
            scope = found.comprehension_parent?;
        }
    }

    /// Bind a name in the current scope.
    pub(super) fn bind(&mut self, name: &str) {
        let scope = &mut self.scopes[self.binder];
        if scope.kind == ScopeKind::Module {
            scope.calls.remove(name);
        }
        scope.bound.insert(name.to_string());
    }

    /// Bind the name of a function or class, given with its index in the definitions of the module, if
    /// it is a local variable of a function or a member of a class. The name of a function or class of
    /// a module is not bound, since it is a definition that is matched by its name.
    pub(super) fn bind_definition(&mut self, name: &str, index: usize) {
        let binder = self.binder;
        let scope = &mut self.scopes[binder];
        let definitions = match scope.kind {
            ScopeKind::Module => {
                scope.calls.remove(name);
                return;
            }
            ScopeKind::Class => &mut scope.members,
            ScopeKind::Function => &mut scope.functions_and_classes,
        };
        definitions.entry(name.to_string()).or_default().push(index);
        self.bind(name);
    }

    /// Bind the names that a statement imports.
    ///
    /// When a name is imported more than once, as by an `import` in a `try` and a fallback in its
    /// `except`, the first import is the one that counts. Within one statement, the last one does.
    pub(super) fn bind_imports(&mut self, statement: FxHashMap<String, Binding>) {
        for (name, binding) in statement {
            self.bind(&name);
            let binder = self.binder;
            self.scopes[binder].imports.entry(name).or_insert(binding);
        }
    }

    /// End a scope: drop the reads of its local variables, resolve the reads of the names that it
    /// imports, which are matched once every module is known, and pass the rest on. A read of a
    /// function or class that is a local variable is a reference to the definitions of that name in
    /// the scope.
    pub(super) fn close(&mut self, inner: usize, local_names: &FxHashSet<String>, target: Target) {
        let reads = std::mem::take(&mut self.scopes[inner].reads);
        let (defined, other): (FxHashMap<String, Vec<usize>>, FxHashMap<_, _>) =
            std::mem::take(&mut self.scopes[inner].functions_and_classes)
                .into_iter()
                .partition(|(name, _)| local_names.contains(name));
        // A function or class whose name is declared `global` is a definition of the module. One whose
        // name is declared `nonlocal`, or is not local in a scope that it was passed on to, is a
        // variable of an enclosing function.
        if let Target::Scope(scope) = target {
            for (name, indexes) in other {
                if !self.scopes[inner].global_names.contains(&name) {
                    self.scopes[scope]
                        .functions_and_classes
                        .entry(name)
                        .or_default()
                        .extend(indexes);
                }
            }
        }
        self.references
            .local_definitions
            .extend(defined.values().flatten().copied());
        let scope = &self.scopes[inner];
        let own_class = scope
            .instance
            .clone()
            .filter(|instance| !scope.bound.contains(instance))
            .zip(scope.class);
        let mut passed = Vec::new();
        for read in reads {
            let root = read.root.as_str();
            if let Some(pattern) = self.scopes[inner].module_patterns.get(root) {
                if let Some(first) = read.attributes.first() {
                    self.references.pattern_reads.insert((
                        read.owner,
                        pattern.clone(),
                        first.clone(),
                    ));
                } else if read.passed && *pattern != ModulePattern::any() {
                    // A module that can be any module of the project is not passed on, as each of
                    // them would be.
                    self.references
                        .passed_patterns
                        .insert((read.owner, pattern.clone()));
                }
            }
            if let Some(binding) = self.scopes[inner].imports.get(root) {
                // A read of a name that is not local, at the module or declared `global`, is also a
                // read of the name of the enclosing scope, without reading the object, as another
                // binding of the name, such as a fallback in an `except ImportError`, can be the one
                // that is read.
                if !local_names.contains(root) {
                    passed.push(Read {
                        owner: read.owner,
                        root: read.root.clone(),
                        attributes: Vec::new(),
                        reads_object: false,
                        called: None,
                        passed: false,
                    });
                }
                // The module that a string imports can be replaced in `sys.modules`, as tests do, so
                // its names are also read from any object.
                if self.scopes[inner].string_imports.contains(root) {
                    let names = read.attributes.iter().chain(&read.called).cloned();
                    self.references
                        .attributes
                        .extend(names.map(|name| (read.owner, name)));
                }
                self.references.imported_reads.insert(ImportedRead {
                    owner: read.owner,
                    binding: binding.clone(),
                    attributes: read.attributes,
                    reads_object: read.reads_object,
                    called: read.called,
                    passed: read.passed,
                });
            } else if local_names.contains(read.root.as_str()) {
                for &index in defined.get(read.root.as_str()).into_iter().flatten() {
                    self.references.local_reads.insert((read.owner, index));
                    if read.reads_object {
                        self.references.local_objects.insert((
                            read.owner,
                            index,
                            read.attributes.clone(),
                        ));
                    }
                }
                let mut attributes = read.attributes.into_iter();
                // The first parameter of a method, unless it is assigned, is the instance or the
                // class.
                if let Some(class) = own_class
                    .as_ref()
                    .filter(|(instance, _)| *instance == read.root)
                    .map(|(_, class)| *class)
                    && let Some(first) = attributes.next()
                {
                    let receiver = Receiver::Own(class);
                    self.references
                        .members
                        .insert((read.owner, receiver, first));
                }
                let names = attributes.chain(read.called);
                self.references
                    .attributes
                    .extend(names.map(|name| (read.owner, name)));
            } else {
                passed.push(read);
            }
        }
        match target {
            Target::Scope(scope) => self.scopes[scope].reads.extend(passed),
            // One at a time, as `extend` would make room for the repeated reads too.
            Target::Module => {
                for read in passed {
                    self.references.reads.insert(read);
                }
            }
        }
    }
}
