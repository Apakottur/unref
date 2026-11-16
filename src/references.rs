//! What a Python module defines and refers to, found by walking its syntax tree once.

use std::path::Path;
use std::sync::Arc;

use ruff_python_ast::visitor::{Visitor, walk_except_handler, walk_expr, walk_pattern, walk_stmt};
use ruff_python_ast::{
    self as ast, CmpOp, ExceptHandler, Expr, ExprContext, Parameters, Pattern, Stmt,
};
use ruff_text_size::Ranged;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::{Definition, Kind, Unwrap, annotation_name, decorator_names, dotted_name};
use crate::positions::SourcePositions;

/// A name that a module imports: the module it is imported from, as it is written, the number of
/// leading dots, and the imported name. The name is None for `import module`, which binds the module
/// itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Binding {
    pub module: Option<String>,
    pub level: u32,
    pub name: Option<String>,
}

/// The code that a reference is made from: the line of the name of the function that the reference
/// is in, or None for code that runs when the module is imported.
pub(crate) type Owner = Option<u32>;

/// A read of a name, or of a chain of attributes that starts with one (`package.module.name`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Read {
    pub owner: Owner,
    pub root: String,
    /// The attributes, in order.
    pub attributes: Vec<String>,
    /// Whether it reads the object, which a type hint of a parameter does not.
    pub reads_object: bool,
}

/// A read of an imported name, or of a chain of attributes that starts with one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ImportedRead {
    pub owner: Owner,
    pub binding: Binding,
    pub attributes: Vec<String>,
    pub reads_object: bool,
}

/// A name that is imported from another module (`from module import name`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Import {
    pub owner: Owner,
    /// The module, as it is written.
    pub module: Option<String>,
    /// The number of leading dots.
    pub level: u32,
    /// The name, `*` for a star import.
    pub name: String,
}

/// The methods of a mapping that read a key, given as their first argument.
const KEY_METHODS: [&str; 3] = ["get", "pop", "setdefault"];

/// What a Python module refers to.
///
/// Each reference is made from an owner: the function that it is in, or the module itself.
#[derive(Debug, Default)]
pub(crate) struct References {
    /// The reads of the names of the module itself, without the local variables and parameters. A name
    /// that is listed in `__all__` is read as a plain name. A plain read of a name that the module
    /// imports, at its top level or declared `global`, also reads its own name, without the object,
    /// such as a fallback definition in an `except ImportError`.
    pub reads: FxHashSet<Read>,
    /// The reads of the imported names, as for `reads`.
    pub imported_reads: FxHashSet<ImportedRead>,
    /// The names that are read as an attribute of something other than a name, such as a call
    /// (`call().name`) or a local variable, and those that a class pattern matches by keyword.
    pub attributes: FxHashSet<(Owner, String)>,
    /// The string keys that are read from a mapping: `value["name"]`, `value.get("name")`,
    /// `"name" in value`, or a mapping pattern (`case {"name": ...}`).
    pub keys: FxHashSet<(Owner, String)>,
    /// The names that are imported from another module. A star import has the name `*`.
    pub imports: FxHashSet<Import>,
    /// Whether the module defines `__all__`.
    pub has_all: bool,
    /// The names that the module imports at its top level, each with what it stands for.
    pub bindings: FxHashMap<String, Binding>,
}

/// Find what a Python module defines and refers to.
///
/// The module is walked once. A read of a name is kept with the scope that it is in until the scope
/// ends, because a name that is bound anywhere in a function is local to all of it. It is then dropped
/// if the name is local, and otherwise passed to the enclosing scope, until it reaches the module.
///
/// The definitions are the functions, classes, methods, variables and attributes of the module. Local
/// variables and imports are left out.
///
/// Returns the definitions, in the order of the file, and the references.
pub(crate) fn walk_module(
    path: &Arc<Path>,
    module: &ast::ModModule,
    positions: &SourcePositions,
) -> (Vec<Definition>, References) {
    let mut walker = Walker {
        path: path.clone(),
        positions,
        definitions: Vec::new(),
        references: References::default(),
        scopes: vec![Scope {
            defined_in: Some(0),
            ..Scope::new(ScopeKind::Module)
        }],
        scope: 0,
        binder: 0,
        owner: None,
        in_signature: false,
    };
    walker.visit_body(&module.body);
    // What reaches the module is read from the module itself, or from what it imports.
    walker.references.bindings = walker.scopes[0].imports.clone();
    walker.close(0, &FxHashSet::default(), Target::Module);
    (walker.definitions, walker.references)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Module,
    Class,
    Function,
}

/// The module, a class body, a function, a lambda or a comprehension, while it is walked.
#[derive(Debug)]
struct Scope {
    /// Whether the definitions in the scope are module variables, class attributes, or local variables
    /// that are left out. A lambda and a comprehension hold no definitions.
    kind: ScopeKind,
    /// The names that the scope binds, which are its local variables.
    bound: FxHashSet<String>,
    /// The names that the scope declares `global`.
    global_names: FxHashSet<String>,
    /// The names that the scope imports, each with what it stands for.
    imports: FxHashMap<String, Binding>,
    /// The reads of names in the scope, or in a scope nested in it, that are not local to the scopes
    /// in between.
    reads: FxHashSet<Read>,
    /// The scope that also binds the names that the scope binds. A comprehension binds its variables
    /// in the function that it is in as well.
    enclosing: Option<usize>,
    /// The scope that holds the variables of a module, or the attributes of a class, that were
    /// already found. A method shares the names of its class, so that an attribute is found once.
    defined_in: Option<usize>,
    /// The variables or attributes that were already found, when the scope holds them.
    defined: FxHashSet<String>,
    /// In a method, the name of its first parameter, usually `self` or `cls`.
    instance: Option<String>,
    /// In a class or a method, the name of the class in the module.
    class_name: Option<String>,
}

impl Scope {
    fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            bound: FxHashSet::default(),
            global_names: FxHashSet::default(),
            imports: FxHashMap::default(),
            reads: FxHashSet::default(),
            enclosing: None,
            defined_in: None,
            defined: FxHashSet::default(),
            instance: None,
            class_name: None,
        }
    }
}

/// Where the reads of a scope that ends go.
#[derive(Clone, Copy)]
enum Target {
    /// The enclosing scope.
    Scope(usize),
    /// The reads of the names of the module itself.
    Module,
}

/// Walk a module once, and add what it defines and refers to.
struct Walker<'p, 'a> {
    /// The file of the module.
    path: Arc<Path>,
    /// The positions in the source of the module.
    positions: &'p SourcePositions<'a>,
    /// The definitions to add to.
    definitions: Vec<Definition>,
    /// The references to add to.
    references: References,
    /// Every scope that was entered, by its index.
    scopes: Vec<Scope>,
    /// The scope that the node being walked is in.
    scope: usize,
    /// The scope that the names bound at the node belong to, which holds its definitions. In a class
    /// body, and in what a function evaluates where it is defined, it is a scope of its own that is
    /// then dropped, since the names that they bind are not tracked.
    binder: usize,
    /// The function that the node is in, or None for code that runs when the module is imported.
    owner: Owner,
    /// Whether the node is in the annotation of a parameter or the return of a function.
    in_signature: bool,
}

impl<'a> Visitor<'a> for Walker<'_, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => self.visit_function_def(function),
            Stmt::ClassDef(class) => self.visit_class(class),
            Stmt::Import(import) => self.visit_import(import),
            Stmt::ImportFrom(import) => self.visit_import_from(import),
            Stmt::Global(global) => {
                // The names are not local variables.
                let binder = self.binder;
                self.scopes[binder]
                    .global_names
                    .extend(global.names.iter().map(ToString::to_string));
            }
            Stmt::Assign(assign) => {
                self.visit_assign(&assign.targets, Some(&assign.value), None, true);
            }
            Stmt::AnnAssign(assign) => self.visit_assign(
                std::slice::from_ref(&*assign.target),
                assign.value.as_deref(),
                Some(&assign.annotation),
                true,
            ),
            Stmt::AugAssign(assign) => self.visit_assign(
                std::slice::from_ref(&*assign.target),
                Some(&assign.value),
                None,
                false,
            ),
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => {
                if name.ctx == ExprContext::Load {
                    self.add_read(name.id.as_str(), Vec::new());
                } else {
                    self.bind(name.id.as_str());
                }
            }
            Expr::Attribute(attribute) => {
                if attribute.ctx != ExprContext::Load {
                    self.visit_expr(&attribute.value);
                    return;
                }
                let (base, attributes) = attribute_chain(attribute);
                if let Expr::Name(name) = base {
                    // Whether the chain starts with an imported name is known when the scope ends.
                    self.add_read(name.id.as_str(), attributes);
                } else {
                    for attribute in attributes {
                        self.references.attributes.insert((self.owner, attribute));
                    }
                    self.visit_expr(base);
                }
            }
            Expr::Subscript(subscript) => {
                // A read of a string key (`value["name"]`).
                if subscript.ctx == ExprContext::Load {
                    self.add_key(&subscript.slice);
                }
                walk_expr(self, expr);
            }
            Expr::Call(call) => {
                // A read of a string key by a method of a mapping (`value.get("name")`).
                if let Expr::Attribute(func) = &*call.func
                    && KEY_METHODS.contains(&func.attr.as_str())
                    && let Some(first) = call.arguments.args.first()
                {
                    self.add_key(first);
                }
                walk_expr(self, expr);
            }
            Expr::Compare(compare) => {
                // A read of a string key by a membership test (`"name" in value`).
                let operands = compare.operands.iter();
                for (operand, operator) in operands.zip(compare.ops.iter()) {
                    if matches!(operator, CmpOp::In | CmpOp::NotIn) {
                        self.add_key(operand);
                    }
                }
                walk_expr(self, expr);
            }
            Expr::Lambda(lambda) => self.visit_function(
                lambda.parameters.as_deref(),
                &[],
                None,
                None,
                FunctionBody::Lambda(&lambda.body),
            ),
            Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_) => {
                // The variables of a comprehension are local to it.
                let (scope, binder) = (self.scope, self.binder);
                let mut inner = Scope::new(ScopeKind::Function);
                inner.enclosing = Some(binder);
                self.scope = self.push_scope(inner);
                self.binder = self.scope;
                walk_expr(self, expr);
                let inner = self.scope;
                self.scope = scope;
                self.binder = binder;
                let local_names = self.scopes[inner].bound.clone();
                self.close(inner, &local_names, Target::Scope(scope));
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ExceptHandler) {
        let ExceptHandler::ExceptHandler(inner) = handler;
        if let Some(name) = &inner.name {
            self.bind(name.as_str());
        }
        walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        match pattern {
            Pattern::MatchAs(ast::PatternMatchAs {
                name: Some(name), ..
            })
            | Pattern::MatchStar(ast::PatternMatchStar {
                name: Some(name), ..
            }) => {
                self.bind(name.as_str());
            }
            Pattern::MatchMapping(mapping) => {
                // A mapping pattern reads its keys.
                for key in &mapping.keys {
                    self.add_key(key);
                }
                if let Some(rest) = &mapping.rest {
                    self.bind(rest.as_str());
                }
            }
            Pattern::MatchClass(class) => {
                // A read of each attribute that a class pattern matches by keyword (`Class(name=pattern)`).
                for keyword in &class.arguments.keywords {
                    self.references
                        .attributes
                        .insert((self.owner, keyword.attr.to_string()));
                }
            }
            _ => {}
        }
        walk_pattern(self, pattern);
    }
}

/// The body of a function or lambda.
#[derive(Clone, Copy)]
enum FunctionBody<'a> {
    Statements(&'a [Stmt]),
    Lambda(&'a Expr),
}

impl<'a> Walker<'_, '_> {
    fn push_scope(&mut self, scope: Scope) -> usize {
        self.scopes.push(scope);
        self.scopes.len() - 1
    }

    fn add_read(&mut self, root: &str, attributes: Vec<String>) {
        let read = Read {
            owner: self.owner,
            root: root.to_string(),
            attributes,
            reads_object: !self.in_signature,
        };
        let scope = self.scope;
        self.scopes[scope].reads.insert(read);
    }

    fn visit_function_def(&mut self, function: &'a ast::StmtFunctionDef) {
        let (body_scope, line) = self.define_function(function);
        self.visit_function(
            Some(&function.parameters),
            &function.decorator_list,
            function.returns.as_deref(),
            Some((body_scope, line)),
            FunctionBody::Statements(&function.body),
        );
    }

    /// Add what a function or lambda refers to.
    fn visit_function(
        &mut self,
        parameters: Option<&'a Parameters>,
        decorators: &'a [ast::Decorator],
        returns: Option<&'a Expr>,
        defined: Option<(Scope, u32)>,
        body: FunctionBody<'a>,
    ) {
        let mut names: Vec<&str> = Vec::new();
        let mut defaults: Vec<&Expr> = Vec::new();
        let mut annotations: Vec<&Expr> = Vec::new();
        if let Some(parameters) = parameters {
            for parameter in parameters {
                names.push(parameter.name().as_str());
                if let Some(default) = parameter.default() {
                    defaults.push(default);
                }
                if let Some(annotation) = parameter.annotation() {
                    annotations.push(annotation);
                }
            }
        }

        // The decorators, the default values and the annotations are evaluated where the function is
        // defined, so they do not see its local variables, and what they bind is not tracked.
        let binder = self.binder;
        self.binder = self.push_scope(Scope::new(ScopeKind::Function));
        for default in defaults {
            self.visit_expr(default);
        }
        for decorator in decorators {
            self.visit_expr(&decorator.expression);
        }
        // A type hint of a parameter or the return reads no value of the object that it names.
        self.in_signature = true;
        for annotation in annotations.into_iter().chain(returns) {
            self.visit_expr(annotation);
        }
        self.in_signature = false;

        // The body runs only when the function is called. A lambda has no name, and is part of the code
        // that it is in.
        let (body_scope, line) = defined.unwrap_or((Scope::new(ScopeKind::Function), 0));
        let (scope, owner) = (self.scope, self.owner);
        self.scope = self.push_scope(body_scope);
        self.binder = self.scope;
        match body {
            FunctionBody::Lambda(expr) => self.visit_expr(expr),
            FunctionBody::Statements(statements) => {
                self.owner = Some(line);
                self.visit_body(statements);
            }
        }
        let inner = self.scope;
        self.scope = scope;
        self.binder = binder;
        self.owner = owner;

        let inner_scope = &self.scopes[inner];
        let mut local_names: FxHashSet<String> =
            names.iter().map(|name| (*name).to_string()).collect();
        local_names.extend(
            inner_scope
                .bound
                .iter()
                .filter(|name| !inner_scope.global_names.contains(*name))
                .cloned(),
        );
        self.close(inner, &local_names, Target::Scope(scope));
    }

    /// Add a class, and what it defines and refers to.
    ///
    /// The body of a class is not a scope whose reads are tracked: it sees the names of the code that
    /// it is in, and what it binds is an attribute of the class.
    fn visit_class(&mut self, class: &'a ast::StmtClassDef) {
        let (line, column) = self.positions.line_column(class.name.start());
        let binder = &self.scopes[self.binder];
        let owner = if binder.kind == ScopeKind::Class {
            binder.class_name.clone()
        } else {
            None
        };
        let bases = class
            .arguments
            .as_deref()
            .map(|arguments| {
                arguments
                    .args
                    .iter()
                    .filter_map(|base| dotted_name(base, Unwrap::Subscript))
                    .collect()
            })
            .unwrap_or_default();
        let definition = Definition {
            path: self.path.clone(),
            line,
            column,
            kind: Kind::Class,
            name: class.name.to_string(),
            decorators: decorator_names(&class.decorator_list),
            owner,
            function: self.owner,
            bases,
            unpacked: false,
            annotation: None,
            call: None,
        };
        let mut class_scope = Scope::new(ScopeKind::Class);
        class_scope.class_name = Some(definition.qualified_name());
        self.definitions.push(definition);

        let binder = self.binder;
        self.binder = self.push_scope(class_scope);
        self.scopes[self.binder].defined_in = Some(self.binder);
        for decorator in &class.decorator_list {
            self.visit_expr(&decorator.expression);
        }
        if let Some(type_params) = &class.type_params {
            self.visit_type_params(type_params);
        }
        if let Some(arguments) = &class.arguments {
            self.visit_arguments(arguments);
        }
        self.visit_body(&class.body);
        self.binder = binder;
    }

    /// Bind the names that an `import` statement imports.
    fn visit_import(&mut self, import: &ast::StmtImport) {
        let bindings = import.names.iter().map(|alias| match &alias.asname {
            // `import package.module` binds `package`.
            None => {
                let package = alias.name.split('.').next().unwrap_or_default().to_string();
                (
                    package.clone(),
                    Binding {
                        module: Some(package),
                        level: 0,
                        name: None,
                    },
                )
            }
            Some(asname) => (
                asname.to_string(),
                Binding {
                    module: Some(alias.name.to_string()),
                    level: 0,
                    name: None,
                },
            ),
        });
        let bindings: Vec<_> = bindings.collect();
        self.bind_imports(bindings);
    }

    /// Add the names that a `from` import imports, and bind them.
    fn visit_import_from(&mut self, import: &ast::StmtImportFrom) {
        let module = import.module.as_ref().map(ToString::to_string);
        let mut bindings = Vec::new();
        for alias in &import.names {
            self.references.imports.insert(Import {
                owner: self.owner,
                module: module.clone(),
                level: import.level,
                name: alias.name.to_string(),
            });
            let bound = alias.asname.as_ref().unwrap_or(&alias.name).to_string();
            bindings.push((
                bound,
                Binding {
                    module: module.clone(),
                    level: import.level,
                    name: Some(alias.name.to_string()),
                },
            ));
        }
        self.bind_imports(bindings);
    }

    /// Add the variables and attributes that an assignment defines, the names that an assignment to
    /// `__all__` lists, and what the assignment refers to.
    fn visit_assign(
        &mut self,
        targets: &'a [Expr],
        value: Option<&'a Expr>,
        annotation: Option<&'a Expr>,
        defines: bool,
    ) {
        if defines {
            let annotation = annotation.map(annotation_name);
            let call = match value {
                Some(Expr::Call(call)) => dotted_name(&call.func, Unwrap::Call),
                _ => None,
            };
            for target in targets {
                self.define_target(target, false, annotation.as_ref(), call.as_ref());
            }
        }
        if targets.iter().any(is_all) {
            self.references.has_all = true;
            for name in string_literals(value) {
                self.references.reads.insert(Read {
                    owner: self.owner,
                    root: name,
                    attributes: Vec::new(),
                    reads_object: true,
                });
            }
        }
        for target in targets {
            self.visit_expr(target);
        }
        if let Some(annotation) = annotation {
            self.visit_expr(annotation);
        }
        if let Some(value) = value {
            self.visit_expr(value);
        }
    }

    /// Add a function or a method.
    ///
    /// Returns the scope of its body, and the line of its name.
    fn define_function(&mut self, function: &ast::StmtFunctionDef) -> (Scope, u32) {
        let enclosing = &self.scopes[self.binder];
        let mut kind = Kind::Function;
        let mut scope = Scope::new(ScopeKind::Function);
        let mut owner = None;
        if enclosing.kind == ScopeKind::Class {
            kind = Kind::Method;
            let parameters = &function.parameters;
            scope.instance = parameters
                .posonlyargs
                .iter()
                .chain(parameters.args.iter())
                .next()
                .map(|parameter| parameter.parameter.name.to_string());
            scope.defined_in = enclosing.defined_in;
            scope.class_name.clone_from(&enclosing.class_name);
            owner.clone_from(&enclosing.class_name);
        }
        let (line, column) = self.positions.line_column(function.name.start());
        self.definitions.push(Definition {
            path: self.path.clone(),
            line,
            column,
            kind,
            name: function.name.to_string(),
            decorators: decorator_names(&function.decorator_list),
            owner,
            function: self.owner,
            bases: Vec::new(),
            unpacked: false,
            annotation: None,
            call: None,
        });
        (scope, line)
    }

    /// Add the variables and attributes that an assignment target defines.
    fn define_target(
        &mut self,
        target: &Expr,
        in_unpacking: bool,
        annotation: Option<&String>,
        call: Option<&String>,
    ) {
        let scope = &self.scopes[self.binder];
        let (kind, name, position, unpacked) = match target {
            Expr::Name(name) if scope.kind != ScopeKind::Function => {
                let kind = if scope.kind == ScopeKind::Module {
                    Kind::Variable
                } else {
                    Kind::Attribute
                };
                (kind, name.id.as_str(), name.start(), in_unpacking)
            }
            Expr::Attribute(attribute)
                if matches!(&*attribute.value, Expr::Name(owner)
                    if scope.instance.as_deref() == Some(owner.id.as_str())) =>
            {
                // The name of the attribute ends the target.
                (
                    Kind::Attribute,
                    attribute.attr.as_str(),
                    attribute.attr.start(),
                    false,
                )
            }
            Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                for element in elts {
                    self.define_target(element, true, None, None);
                }
                return;
            }
            Expr::Starred(starred) => {
                self.define_target(&starred.value, true, None, None);
                return;
            }
            // A local variable, or an assignment into an object that is defined elsewhere.
            _ => return,
        };

        // A variable or attribute is found once, where it is first assigned.
        let class_name = scope.class_name.clone();
        let Some(defined_in) = scope.defined_in else {
            return;
        };
        if !self.scopes[defined_in].defined.insert(name.to_string()) {
            return;
        }
        let (line, column) = self.positions.line_column(position);
        self.definitions.push(Definition {
            path: self.path.clone(),
            line,
            column,
            kind,
            name: name.to_string(),
            decorators: Vec::new(),
            owner: class_name,
            function: self.owner,
            bases: Vec::new(),
            unpacked,
            annotation: annotation.cloned(),
            call: call.cloned(),
        });
    }

    /// Add a read of a key, if it is a string literal.
    fn add_key(&mut self, key: &Expr) {
        if let Expr::StringLiteral(literal) = key {
            self.references
                .keys
                .insert((self.owner, literal.value.to_str().to_string()));
        }
    }

    /// Bind a name in the current scope, and in the scopes that it binds names in as well.
    fn bind(&mut self, name: &str) {
        let mut scope = Some(self.binder);
        while let Some(index) = scope {
            self.scopes[index].bound.insert(name.to_string());
            scope = self.scopes[index].enclosing;
        }
    }

    /// Bind the names that a statement imports.
    ///
    /// When a name is imported more than once, as by an `import` in a `try` and a fallback in its
    /// `except`, the first import is the one that counts. Within one statement, the last one does.
    fn bind_imports(&mut self, bindings: Vec<(String, Binding)>) {
        let mut statement: FxHashMap<String, Binding> = FxHashMap::default();
        for (name, binding) in bindings {
            statement.insert(name, binding);
        }
        for (name, binding) in statement {
            self.bind(&name);
            let binder = self.binder;
            self.scopes[binder].imports.entry(name).or_insert(binding);
        }
    }

    /// End a scope: drop the reads of its local variables, resolve the reads of the names that it
    /// imports, which are matched once every module is known, and pass the rest on.
    fn close(&mut self, inner: usize, local_names: &FxHashSet<String>, target: Target) {
        let reads = std::mem::take(&mut self.scopes[inner].reads);
        let mut passed = Vec::new();
        for read in reads {
            if let Some(binding) = self.scopes[inner].imports.get(&read.root) {
                // A plain read of a name that is not local, at the module or declared `global`, is also
                // a read of the name of the enclosing scope, without reading the object.
                if read.attributes.is_empty() && !local_names.contains(&read.root) {
                    passed.push(Read {
                        owner: read.owner,
                        root: read.root.clone(),
                        attributes: Vec::new(),
                        reads_object: false,
                    });
                }
                self.references.imported_reads.insert(ImportedRead {
                    owner: read.owner,
                    binding: binding.clone(),
                    attributes: read.attributes,
                    reads_object: read.reads_object,
                });
            } else if local_names.contains(&read.root) {
                for attribute in read.attributes {
                    self.references.attributes.insert((read.owner, attribute));
                }
            } else {
                passed.push(read);
            }
        }
        match target {
            Target::Scope(scope) => self.scopes[scope].reads.extend(passed),
            Target::Module => self.references.reads.extend(passed),
        }
    }
}

/// Split a chain of attributes, such as `package.module.name`, into what it starts with and its
/// attributes in order.
fn attribute_chain(attribute: &ast::ExprAttribute) -> (&Expr, Vec<String>) {
    let mut attributes = vec![attribute.attr.to_string()];
    let mut node = &*attribute.value;
    while let Expr::Attribute(inner) = node {
        attributes.push(inner.attr.to_string());
        node = &inner.value;
    }
    attributes.reverse();
    (node, attributes)
}

/// Check whether an assignment target is the `__all__` variable.
fn is_all(target: &Expr) -> bool {
    matches!(target, Expr::Name(name) if name.id.as_str() == "__all__")
}

/// Get the string literals of a list or tuple literal. Anything else, such as a computed list, has
/// none.
fn string_literals(value: Option<&Expr>) -> Vec<String> {
    let elements = match value {
        Some(Expr::List(list)) => &list.elts,
        Some(Expr::Tuple(tuple)) => &tuple.elts,
        _ => return Vec::new(),
    };
    elements
        .iter()
        .filter_map(|element| match element {
            Expr::StringLiteral(literal) => Some(literal.value.to_str().to_string()),
            _ => None,
        })
        .collect()
}
