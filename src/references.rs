//! What a Python module defines and refers to, found by walking its syntax tree once.

use std::path::Path;
use std::sync::Arc;

use ruff_python_ast::visitor::{
    Visitor, walk_except_handler, walk_expr, walk_pattern, walk_stmt, walk_type_param,
    walk_with_item,
};
use ruff_python_ast::{
    self as ast, CmpOp, ExceptHandler, Expr, ExprContext, Operator, Parameters, Pattern, Stmt,
    TypeParam,
};
use ruff_text_size::{Ranged, TextSize};
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

/// The code that a reference or a definition is in: the line of the name of the function that it is
/// in, or None for code that runs when the module is imported.
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
    /// The attribute read from the result of calling the chain, such as `name` in `Class().name`.
    pub called: Option<String>,
}

/// A read of an imported name, or of a chain of attributes that starts with one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ImportedRead {
    pub owner: Owner,
    pub binding: Binding,
    pub attributes: Vec<String>,
    pub reads_object: bool,
    pub called: Option<String>,
}

/// The object that a member is read from, when the class that it is in is plain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Receiver {
    /// The first parameter of a method of the class, as `self.name` or `cls.name`, by the name of the
    /// class in the module.
    Own(String),
    /// `super()` in a method of the class, or `super(Class, self)`, by the name of the class in the
    /// module.
    Super(String),
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
/// The functions and methods whose string arguments frameworks read: Django's `include`, which names a
/// module of URLs, and `os.environ.setdefault`, which can set `DJANGO_SETTINGS_MODULE`.
const STRING_CALLS: [&str; 2] = ["include", "setdefault"];

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
    /// The names that are read as a member of the class of a method, through its first parameter or
    /// `super()`.
    pub members: FxHashSet<(Owner, Receiver, String)>,
    /// The string keys that are read from a mapping: `value["name"]`, `value.get("name")`,
    /// `"name" in value`, or a mapping pattern (`case {"name": ...}`).
    pub keys: FxHashSet<(Owner, String)>,
    /// The names that are imported from another module. A star import has the name `*`.
    pub imports: FxHashSet<Import>,
    /// Whether the module defines `__all__`.
    pub has_all: bool,
    /// The names that the module imports at its top level, each with what it stands for.
    pub bindings: FxHashMap<String, Binding>,
    /// The functions and classes that are local variables of a function, by their index in the
    /// definitions of the module. A read of their name elsewhere is not a reference to them.
    pub local_definitions: FxHashSet<usize>,
    /// The reads of the functions and classes that are local variables of a function, and of the
    /// members of a class in its class body, each with the index of the definition in the definitions
    /// of the module.
    pub local_reads: FxHashSet<(Owner, usize)>,
    /// The local variables of the functions that are assigned the result of a call.
    pub local_calls: LocalCalls,
    /// The annotated parameters of the functions, by the line of the name of the function and the
    /// name of the parameter, each with the dotted name of its annotation as it is written, without
    /// its type arguments, such as `Flask` for `app: Flask`.
    pub annotated_parameters: FxHashMap<(u32, String), String>,
    /// The calls of the functions and methods whose string arguments frameworks read, each by the
    /// dotted name of what it calls as it is written, such as `os.environ.setdefault`, and its leading
    /// positional arguments that are string literals.
    pub string_calls: FxHashSet<(String, Vec<String>)>,
}

/// The local variables of each function of a module that are assigned the result of a call, by the
/// line of the name of the function and the name of the variable, each with the dotted names of what
/// they call as they are written, such as `FastAPI` for `app = FastAPI()`.
pub(crate) type LocalCalls = FxHashMap<(u32, String), Vec<String>>;

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
    walker.close(0, &FxHashSet::default(), Target::Module);
    walker.references.bindings = std::mem::take(&mut walker.scopes[0].imports);
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
    /// The names that the scope declares `nonlocal`.
    nonlocal_names: FxHashSet<String>,
    /// The names that the scope imports, each with what it stands for.
    imports: FxHashMap<String, Binding>,
    /// In a function, the functions and classes that it defines, by their name, each with its index
    /// in the definitions of the module.
    functions_and_classes: FxHashMap<String, Vec<usize>>,
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
    /// In a class body, the attributes, methods and classes that it defines so far, by their name,
    /// each with its index in the definitions of the module.
    members: FxHashMap<String, Vec<usize>>,
    /// In a function, the local variables that are assigned the result of a call, each with the
    /// dotted names of what they call as they are written.
    calls: FxHashMap<String, Vec<String>>,
    /// In a class body, the nearest scope around it that is not a class body. The functions,
    /// lambdas and comprehensions in a class body do not see its names, and neither does a class
    /// body nested in it, so their reads go on to this scope, as do the reads of the class body that
    /// are not of its own names.
    outer: Option<usize>,
}

impl Scope {
    fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            bound: FxHashSet::default(),
            global_names: FxHashSet::default(),
            nonlocal_names: FxHashSet::default(),
            imports: FxHashMap::default(),
            functions_and_classes: FxHashMap::default(),
            reads: FxHashSet::default(),
            enclosing: None,
            defined_in: None,
            defined: FxHashSet::default(),
            instance: None,
            class_name: None,
            members: FxHashMap::default(),
            calls: FxHashMap::default(),
            outer: None,
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
    /// The scope that the names bound at the node belong to, which holds its definitions. It is the
    /// scope of the node, except in what a class or function evaluates where it is defined. The
    /// decorators, type parameters and base classes of a class bind in the class body. The decorators,
    /// default values and annotations of a function bind in a scope of their own that is then dropped,
    /// since the names that they bind are not tracked.
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
            Stmt::Nonlocal(nonlocal) => {
                // The names are variables of an enclosing function.
                let binder = self.binder;
                self.scopes[binder]
                    .nonlocal_names
                    .extend(nonlocal.names.iter().map(ToString::to_string));
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
            Stmt::TypeAlias(alias) => {
                // Its value is evaluated when it is used, but what it reads is still read.
                self.define_target(&alias.name, false, None, None, None);
                walk_stmt(self, stmt);
                self.visit_type_strings(&alias.value);
            }
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_with_item(&mut self, item: &'a ast::WithItem) {
        // The target of `with ... as target` is assigned, as by an assignment. The target of a `for`
        // loop is not collected: like Ruff, which leaves it to a rule of its own, unref does not count
        // an unused loop variable as an unused variable.
        if let Some(target) = &item.optional_vars {
            self.define_target(target, false, None, None, None);
        }
        walk_with_item(self, item);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => {
                if name.ctx == ExprContext::Load {
                    self.add_read(name.id.as_str(), Vec::new(), None);
                } else {
                    self.bind(name.id.as_str());
                }
            }
            Expr::Attribute(attribute) => {
                if attribute.ctx != ExprContext::Load {
                    self.visit_expr(&attribute.value);
                    return;
                }
                let (base, mut attributes) = attribute_chain(attribute);
                match base {
                    // Whether the chain starts with an imported name is known when the scope ends.
                    Expr::Name(name) => self.add_read(name.id.as_str(), attributes, None),
                    Expr::Call(call) => {
                        let first = attributes.remove(0);
                        self.add_attributes(attributes);
                        if let Some(class) = self.super_class(call) {
                            self.references.members.insert((
                                self.owner,
                                Receiver::Super(class),
                                first,
                            ));
                            walk_expr(self, base);
                        } else {
                            self.visit_call(call, Some(first));
                        }
                    }
                    _ => {
                        self.add_attributes(attributes);
                        self.visit_expr(base);
                    }
                }
            }
            Expr::Subscript(subscript) => {
                // A read of a string key (`value["name"]`).
                if subscript.ctx == ExprContext::Load {
                    self.add_key(&subscript.slice);
                }
                walk_expr(self, expr);
            }
            Expr::Call(call) => self.visit_call(call, None),
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
            Expr::ListComp(ast::ExprListComp {
                elt, generators, ..
            })
            | Expr::SetComp(ast::ExprSetComp {
                elt, generators, ..
            })
            | Expr::Generator(ast::ExprGenerator {
                elt, generators, ..
            }) => self.visit_comprehension(&[elt], generators),
            Expr::DictComp(ast::ExprDictComp {
                key,
                value,
                generators,
                ..
            }) => {
                let elements: Vec<&Expr> = key.as_deref().into_iter().chain([&**value]).collect();
                self.visit_comprehension(&elements, generators);
            }
            Expr::Named(named) => {
                // The target of a walrus is assigned, as by an assignment.
                self.define_target(&named.target, false, None, None, None);
                walk_expr(self, expr);
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_type_param(&mut self, type_param: &'a TypeParam) {
        walk_type_param(self, type_param);
        // The bound and the default of a type parameter are types.
        let (bound, default) = match type_param {
            TypeParam::TypeVar(type_var) => {
                (type_var.bound.as_deref(), type_var.default.as_deref())
            }
            TypeParam::ParamSpec(spec) => (None, spec.default.as_deref()),
            TypeParam::TypeVarTuple(tuple) => (None, tuple.default.as_deref()),
        };
        for expr in bound.into_iter().chain(default) {
            self.visit_type_strings(expr);
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
                let names = class.arguments.keywords.iter();
                self.add_attributes(names.map(|keyword| keyword.attr.to_string()));
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

    /// Add a read of a name, or of a chain of attributes that starts with one.
    ///
    /// In a class body, a name that the body has bound so far is read from the class: it is a
    /// reference to the attributes, methods and classes of that name in the class, and to nothing else.
    /// Python reads a name of a class body that is not bound yet from the code around the class, so
    /// that read is passed on.
    ///
    /// `called` is the attribute read from the result of calling the chain, as in `Class().name`.
    fn add_read(&mut self, root: &str, attributes: Vec<String>, called: Option<String>) {
        let scope = &self.scopes[self.scope];
        if scope.kind == ScopeKind::Class
            && scope.bound.contains(root)
            && !scope.global_names.contains(root)
            && !scope.imports.contains_key(root)
        {
            let owner = self.owner;
            let members = scope.members.get(root).into_iter().flatten();
            self.references
                .local_reads
                .extend(members.map(|&index| (owner, index)));
            self.add_attributes(attributes.into_iter().chain(called));
            return;
        }
        let read = Read {
            owner: self.owner,
            root: root.to_string(),
            attributes,
            reads_object: !self.in_signature,
            called,
        };
        let scope = self.scope;
        self.scopes[scope].reads.insert(read);
    }

    /// Add the names that are read as attributes of an object whose class is not known.
    fn add_attributes(&mut self, names: impl IntoIterator<Item = String>) {
        let owner = self.owner;
        self.references
            .attributes
            .extend(names.into_iter().map(|name| (owner, name)));
    }

    /// Add what a call refers to.
    ///
    /// `called` is the attribute read from its result, as in `Class().name`. It is read from the
    /// class when what is called is a name, or a chain of attributes that starts with one.
    fn visit_call(&mut self, call: &'a ast::ExprCall, called: Option<String>) {
        // A read of a string key by a method of a mapping (`value.get("name")`).
        if let Expr::Attribute(func) = &*call.func
            && KEY_METHODS.contains(&func.attr.as_str())
            && let Some(first) = call.arguments.args.first()
        {
            self.add_key(first);
        }
        self.add_string_call(call);
        let (base, attributes) = match &*call.func {
            Expr::Attribute(func) => attribute_chain(func),
            func => (func, Vec::new()),
        };
        if let Expr::Name(name) = base {
            self.add_read(name.id.as_str(), attributes, called);
        } else {
            self.add_attributes(called);
            self.visit_expr(&call.func);
        }
        self.visit_arguments(&call.arguments);
        for argument in type_arguments(call) {
            self.visit_type_strings(argument);
        }
    }

    /// Add the reads of the names in the strings of a type expression that are types themselves, as
    /// Python reads them when it evaluates the type: a quoted annotation (`"Item"`), and a string in
    /// a subscript or a union of a type (`list["Item"]`, `int | "Item"`). The names are read in the
    /// scope of the type, as when they are not quoted. A string that is not a valid expression reads
    /// nothing.
    fn visit_type_strings(&mut self, expr: &Expr) {
        let mut strings = Vec::new();
        type_strings(expr, &mut strings);
        for string in strings {
            let Ok(parsed) = ruff_python_parser::parse_expression(string.value.to_str().trim())
            else {
                continue;
            };
            let mut names = TypeNames::default();
            names.visit_expr(parsed.expr());
            for (root, attributes) in names.reads {
                self.add_read(&root, attributes, None);
            }
            self.add_attributes(names.attributes);
            self.visit_type_strings(parsed.expr());
        }
    }

    /// Get the class whose base classes a call of `super` reads from: `super()` in a method, or
    /// `super(Class, self)` in a method of `Class`.
    fn super_class(&self, call: &ast::ExprCall) -> Option<String> {
        let Expr::Name(func) = &*call.func else {
            return None;
        };
        let scope = &self.scopes[self.scope];
        let class = scope.class_name.as_ref()?;
        // Only the body of a method has an instance.
        if func.id.as_str() != "super" || scope.instance.is_none() {
            return None;
        }
        let arguments = &call.arguments;
        let plain = match (arguments.args.as_ref(), arguments.keywords.is_empty()) {
            ([], true) => true,
            ([Expr::Name(first), Expr::Name(second)], true) => {
                class.rsplit('.').next() == Some(first.id.as_str())
                    && scope.instance.as_deref() == Some(second.id.as_str())
            }
            _ => false,
        };
        plain.then(|| class.clone())
    }

    /// Add what a comprehension refers to.
    ///
    /// The variables of a comprehension are local to it. Its first iterable is evaluated in the scope
    /// that the comprehension is in, so it sees the names of a class body, and the rest does not.
    fn visit_comprehension(&mut self, elements: &[&'a Expr], generators: &'a [ast::Comprehension]) {
        let Some(first) = generators.first() else {
            return;
        };
        self.visit_expr(&first.iter);
        let (scope, binder) = (self.scope, self.binder);
        let mut inner = Scope::new(ScopeKind::Function);
        // A walrus binds its name in the function that the comprehension is in as well. It cannot be
        // used in a comprehension in a class body.
        if self.scopes[binder].kind != ScopeKind::Class {
            inner.enclosing = Some(binder);
        }
        self.scope = self.push_scope(inner);
        self.binder = self.scope;
        for (index, generator) in generators.iter().enumerate() {
            if index > 0 {
                self.visit_expr(&generator.iter);
            }
            self.visit_expr(&generator.target);
            for condition in &generator.ifs {
                self.visit_expr(condition);
            }
        }
        for element in elements {
            self.visit_expr(element);
        }
        let inner = self.scope;
        self.scope = scope;
        self.binder = binder;
        let local_names = self.scopes[inner].bound.clone();
        let target = self.reads_target(scope);
        self.close(inner, &local_names, Target::Scope(target));
    }

    /// Get the scope that the reads of a function, lambda or comprehension that is in a scope go to:
    /// the scope itself, or for a class body, the nearest scope around it that is not a class body.
    fn reads_target(&self, scope: usize) -> usize {
        self.scopes[scope].outer.unwrap_or(scope)
    }

    fn visit_function_def(&mut self, function: &'a ast::StmtFunctionDef) {
        let index = self.definitions.len();
        let (body_scope, line) = self.define_function(function);
        // The bounds and defaults of the type parameters are read where the function is defined.
        if let Some(type_params) = &function.type_params {
            self.visit_type_params(type_params);
        }
        self.visit_function(
            Some(&function.parameters),
            &function.decorator_list,
            function.returns.as_deref(),
            Some((body_scope, line)),
            FunctionBody::Statements(&function.body),
        );
        // The name is bound once the function is defined, after its decorators are evaluated.
        self.bind_definition(function.name.as_str(), index);
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
                    if let Some((_, line)) = &defined {
                        let name = annotation_name(annotation);
                        if !name.is_empty() {
                            self.references
                                .annotated_parameters
                                .insert((*line, parameter.name().to_string()), name);
                        }
                    }
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
            self.visit_type_strings(annotation);
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

        let inner_scope = &mut self.scopes[inner];
        // A variable declared `global` or `nonlocal` is not local to the function.
        let mut local_names: FxHashSet<String> = inner_scope
            .bound
            .iter()
            .filter(|name| {
                !inner_scope.global_names.contains(*name)
                    && !inner_scope.nonlocal_names.contains(*name)
            })
            .cloned()
            .collect();
        // A variable that is assigned the result of a call is bound, so it is local unless declared
        // `global` or `nonlocal`.
        let calls = std::mem::take(&mut inner_scope.calls);
        if line != 0 {
            let calls = calls
                .into_iter()
                .filter(|(name, _)| local_names.contains(name));
            self.references
                .local_calls
                .extend(calls.map(|(name, called)| ((line, name), called)));
        }
        local_names.extend(names.iter().map(|name| (*name).to_string()));
        let target = self.reads_target(scope);
        self.close(inner, &local_names, Target::Scope(target));
    }

    /// Add a class, and what it defines and refers to.
    ///
    /// The body of a class is a scope whose names are the attributes, methods and classes of the class.
    /// The functions in it do not see them.
    fn visit_class(&mut self, class: &'a ast::StmtClassDef) {
        let binder = &self.scopes[self.binder];
        let enclosing_class = if binder.kind == ScopeKind::Class {
            binder.class_name.clone()
        } else {
            None
        };
        let written_bases = class
            .arguments
            .as_deref()
            .map(|arguments| &*arguments.args)
            .unwrap_or_default();
        let bases: Vec<String> = written_bases
            .iter()
            .filter_map(|base| dotted_name(base, Unwrap::Subscript))
            .collect();
        let other_bases = bases.len() < written_bases.len();
        let definition = Definition {
            decorators: decorator_names(&class.decorator_list),
            class: enclosing_class,
            bases,
            other_bases,
            ..self.definition(Kind::Class, &class.name, class.name.start())
        };
        let mut class_scope = Scope::new(ScopeKind::Class);
        class_scope.class_name = Some(definition.qualified_name());
        class_scope.outer = Some(self.reads_target(self.scope));
        let index = self.definitions.len();
        self.definitions.push(definition);

        let (scope, binder) = (self.scope, self.binder);
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
        self.scope = self.binder;
        self.visit_body(&class.body);
        let inner = self.scope;
        self.scope = scope;
        self.binder = binder;

        // The names that the class body imports are resolved in it, and its other reads are of names
        // that it had not bound yet, so they go on.
        let imported = self.scopes[inner].imports.keys().cloned().collect();
        let target = self.reads_target(inner);
        self.close(inner, &imported, Target::Scope(target));
        // The name is bound once the class is defined.
        self.bind_definition(class.name.as_str(), index);
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
        let bindings = bindings.collect();
        self.bind_imports(bindings);
    }

    /// Add the names that a `from` import imports, and bind them.
    fn visit_import_from(&mut self, import: &ast::StmtImportFrom) {
        let module = import.module.as_ref().map(ToString::to_string);
        let mut bindings = FxHashMap::default();
        for alias in &import.names {
            self.references.imports.insert(Import {
                owner: self.owner,
                module: module.clone(),
                level: import.level,
                name: alias.name.to_string(),
            });
            let bound = alias.asname.as_ref().unwrap_or(&alias.name).to_string();
            bindings.insert(
                bound,
                Binding {
                    module: module.clone(),
                    level: import.level,
                    name: Some(alias.name.to_string()),
                },
            );
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
            let string = match value {
                Some(Expr::StringLiteral(literal)) => Some(literal.value.to_str().to_string()),
                _ => None,
            };
            for target in targets {
                self.define_target(
                    target,
                    false,
                    annotation.as_ref(),
                    call.as_ref(),
                    string.as_ref(),
                );
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
                    called: None,
                });
            }
        }
        // The value is evaluated before the targets are bound, so in a class body, `size = size + 1`
        // reads the name around the class.
        if let Some(value) = value {
            self.visit_expr(value);
        }
        if let Some(annotation) = annotation {
            self.visit_expr(annotation);
            self.visit_type_strings(annotation);
            // The value of `Alias: TypeAlias = ...` is a type.
            let name = dotted_name(annotation, Unwrap::Subscript);
            if let Some(value) = value
                && name.is_some_and(|name| typing_name(&name) == Some("TypeAlias"))
            {
                self.visit_type_strings(value);
            }
        }
        for target in targets {
            self.visit_expr(target);
        }
    }

    /// Add a function or a method.
    ///
    /// Returns the scope of its body, and the line of its name.
    fn define_function(&mut self, function: &ast::StmtFunctionDef) -> (Scope, u32) {
        let enclosing = &self.scopes[self.binder];
        let mut kind = Kind::Function;
        let mut scope = Scope::new(ScopeKind::Function);
        let mut class = None;
        if enclosing.kind == ScopeKind::Class {
            kind = Kind::Method;
            let parameters = &function.parameters;
            // The first parameter of a static method is not the instance or the class.
            let is_static = function.decorator_list.iter().any(|decorator| {
                matches!(&decorator.expression, Expr::Name(name) if name.id.as_str() == "staticmethod")
            });
            scope.instance = parameters
                .posonlyargs
                .iter()
                .chain(parameters.args.iter())
                .next()
                .filter(|_| !is_static)
                .map(|parameter| parameter.parameter.name.to_string());
            scope.defined_in = enclosing.defined_in;
            scope.class_name.clone_from(&enclosing.class_name);
            class.clone_from(&enclosing.class_name);
        }
        let definition = Definition {
            decorators: decorator_names(&function.decorator_list),
            class,
            ..self.definition(kind, &function.name, function.name.start())
        };
        let line = definition.line;
        self.definitions.push(definition);
        (scope, line)
    }

    /// Add the variables and attributes that an assignment target defines.
    fn define_target(
        &mut self,
        target: &Expr,
        in_unpacking: bool,
        annotation: Option<&String>,
        call: Option<&String>,
        string: Option<&String>,
    ) {
        if let (Expr::Name(name), Some(call)) = (target, call) {
            let binder = self.binder;
            if self.scopes[binder].kind == ScopeKind::Function {
                self.scopes[binder]
                    .calls
                    .entry(name.id.to_string())
                    .or_default()
                    .push(call.clone());
            }
        }
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
                    self.define_target(element, true, None, None, None);
                }
                return;
            }
            Expr::Starred(starred) => {
                self.define_target(&starred.value, true, None, None, None);
                return;
            }
            // A local variable, or an assignment into an object that is defined elsewhere.
            _ => return,
        };

        // A variable or attribute is found once, where it is first assigned.
        let class_name = scope.class_name.clone();
        let in_class_body = scope.kind == ScopeKind::Class;
        let Some(defined_in) = scope.defined_in else {
            return;
        };
        if !self.scopes[defined_in].defined.insert(name.to_string()) {
            return;
        }
        if in_class_body {
            let index = self.definitions.len();
            let binder = self.binder;
            self.scopes[binder]
                .members
                .entry(name.to_string())
                .or_default()
                .push(index);
        }
        self.definitions.push(Definition {
            class: class_name,
            unpacked,
            annotation: annotation.cloned(),
            call: call.cloned(),
            string: string.cloned(),
            ..self.definition(kind, name, position)
        });
    }

    /// Start a definition of a name at a position, in the function that the node is in, with no
    /// decorators, class, base classes, annotation, call or string, and not unpacked.
    fn definition(&self, kind: Kind, name: &str, position: TextSize) -> Definition {
        let (line, column) = self.positions.line_column(position);
        Definition {
            path: self.path.clone(),
            line,
            column,
            kind,
            name: name.to_string(),
            decorators: Vec::new(),
            class: None,
            owner: self.owner,
            bases: Vec::new(),
            other_bases: false,
            unpacked: false,
            annotation: None,
            call: None,
            string: None,
        }
    }

    /// Add a call of a function or method whose string arguments frameworks read, with its leading
    /// positional arguments that are string literals.
    fn add_string_call(&mut self, call: &ast::ExprCall) {
        let Some(called) = dotted_name(&call.func, Unwrap::Call) else {
            return;
        };
        let last = called.rsplit('.').next().unwrap_or_default();
        if !STRING_CALLS.contains(&last) {
            return;
        }
        let arguments = call
            .arguments
            .args
            .iter()
            .map_while(|argument| match argument {
                Expr::StringLiteral(literal) => Some(literal.value.to_str().to_string()),
                _ => None,
            })
            .collect();
        self.references.string_calls.insert((called, arguments));
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

    /// Bind the name of a function or class, given with its index in the definitions of the module, if
    /// it is a local variable of a function or a member of a class. The name of a function or class of
    /// a module is not bound, since it is a definition that is matched by its name.
    fn bind_definition(&mut self, name: &str, index: usize) {
        let binder = self.binder;
        let scope = &mut self.scopes[binder];
        let definitions = match scope.kind {
            ScopeKind::Module => return,
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
    fn bind_imports(&mut self, statement: FxHashMap<String, Binding>) {
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
    fn close(&mut self, inner: usize, local_names: &FxHashSet<String>, target: Target) {
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
            .zip(scope.class_name.clone());
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
                        called: None,
                    });
                }
                self.references.imported_reads.insert(ImportedRead {
                    owner: read.owner,
                    binding: binding.clone(),
                    attributes: read.attributes,
                    reads_object: read.reads_object,
                    called: read.called,
                });
            } else if local_names.contains(&read.root) {
                for &index in defined.get(&read.root).into_iter().flatten() {
                    self.references.local_reads.insert((read.owner, index));
                }
                let mut attributes = read.attributes.into_iter();
                // The first parameter of a method, unless it is assigned, is the instance or the
                // class.
                if let Some(class) = own_class
                    .as_ref()
                    .filter(|(instance, _)| *instance == read.root)
                    .map(|(_, class)| class)
                    && let Some(first) = attributes.next()
                {
                    let receiver = Receiver::Own(class.clone());
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

/// The names that a type expression parsed from a string reads.
#[derive(Default)]
struct TypeNames {
    /// The names, or the chains of attributes that start with one, each with its attributes in order.
    reads: Vec<(String, Vec<String>)>,
    /// The names that are read as attributes of something other than a name.
    attributes: Vec<String>,
}

impl<'b> Visitor<'b> for TypeNames {
    fn visit_expr(&mut self, expr: &'b Expr) {
        match expr {
            Expr::Name(name) => self.reads.push((name.id.to_string(), Vec::new())),
            Expr::Attribute(attribute) => {
                let (base, attributes) = attribute_chain(attribute);
                if let Expr::Name(name) = base {
                    self.reads.push((name.id.to_string(), attributes));
                } else {
                    self.attributes.extend(attributes);
                    self.visit_expr(base);
                }
            }
            _ => walk_expr(self, expr),
        }
    }
}

/// Get the strings of a type expression that are types themselves: the expression itself when it is
/// a string, and the strings in its subscripts, unions, tuples, lists and unpacked types, such as
/// `"Item"` in `list["Item"]`, `int | "Item"`, `Callable[["Item"], None]` or `*tuple["Item"]`. The
/// values of `Literal` are not types, and of `Annotated[T, ...]`, only `T` is.
fn type_strings<'e>(expr: &'e Expr, strings: &mut Vec<&'e ast::ExprStringLiteral>) {
    match expr {
        Expr::StringLiteral(string) => strings.push(string),
        Expr::Subscript(subscript) => {
            let name = dotted_name(&subscript.value, Unwrap::Subscript);
            match name.as_deref().and_then(typing_name) {
                Some("Literal") => {}
                Some("Annotated") => match &*subscript.slice {
                    Expr::Tuple(tuple) => {
                        if let Some(first) = tuple.elts.first() {
                            type_strings(first, strings);
                        }
                    }
                    slice => type_strings(slice, strings),
                },
                _ => type_strings(&subscript.slice, strings),
            }
        }
        Expr::BinOp(binary) if binary.op == Operator::BitOr => {
            type_strings(&binary.left, strings);
            type_strings(&binary.right, strings);
        }
        Expr::Starred(starred) => type_strings(&starred.value, strings),
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            for element in elts {
                type_strings(element, strings);
            }
        }
        _ => {}
    }
}

/// Get the arguments of a call that are types: the first argument of `typing.cast`, the constraints,
/// the bound and the default of `typing.TypeVar`, and the second argument of `typing.NewType`.
fn type_arguments(call: &ast::ExprCall) -> Vec<&Expr> {
    let Some(name) = dotted_name(&call.func, Unwrap::Subscript) else {
        return Vec::new();
    };
    let arguments = &call.arguments;
    let (positional, keywords): (_, &[&str]) = match typing_name(&name) {
        Some("cast") => (arguments.args.get(..1), &["typ"]),
        Some("TypeVar") => (arguments.args.get(1..), &["bound", "default"]),
        Some("NewType") => (arguments.args.get(1..2), &["tp"]),
        _ => return Vec::new(),
    };
    let keyword = arguments.keywords.iter().filter(|keyword| {
        keyword
            .arg
            .as_ref()
            .is_some_and(|arg| keywords.contains(&arg.as_str()))
    });
    positional
        .unwrap_or_default()
        .iter()
        .chain(keyword.map(|keyword| &keyword.value))
        .collect()
}

/// Get the name of a member of `typing` or `typing_extensions` from the dotted name that it is read
/// by, such as `cast` for `typing.cast` or `cast`. Returns None for a member of another module.
fn typing_name(name: &str) -> Option<&str> {
    match name.rsplit_once('.') {
        None => Some(name),
        Some(("typing" | "typing_extensions", member)) => Some(member),
        Some(_) => None,
    }
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
