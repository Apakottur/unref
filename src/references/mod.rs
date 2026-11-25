//! What a Python module defines and refers to, found by walking its syntax tree once.

mod definitions;
mod dynamic_imports;
mod hints;
mod imports;
mod nested;
mod reads;
mod scopes;
mod type_strings;
mod types;

use ruff_python_ast::visitor::{
    Visitor, walk_except_handler, walk_expr, walk_pattern, walk_stmt, walk_type_param,
    walk_with_item,
};
use ruff_python_ast::{
    self as ast, CmpOp, ExceptHandler, Expr, ExprContext, Pattern, Stmt, TypeParam,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::Definition;
use crate::positions::SourcePositions;

use definitions::all_call_names;
use nested::FunctionBody;
use reads::attribute_chain;
use scopes::{Scope, ScopeKind, Target};

pub(crate) use types::{
    Binding, Hints, Import, ImportedRead, Index, ModulePattern, Owner, Read, Receiver, References,
    WholeRead,
};

/// Find what a Python module defines and refers to.
///
/// The module is walked once. A read of a name is kept with the scope that it is in until the scope
/// ends, because a name that is bound anywhere in a function is local to all of it. It is then dropped
/// if the name is local, and otherwise passed to the enclosing scope, until it reaches the module.
///
/// The definitions are the functions, classes, methods, variables and attributes of the module. Local
/// variables and imports are left out.
///
/// Returns the definitions, in the order of the file, the references, and the hints for frameworks.
pub(crate) fn walk_module(
    module: &ast::ModModule,
    positions: &SourcePositions,
) -> (Vec<Definition>, References, Hints) {
    let mut walker = Walker {
        positions,
        definitions: Vec::new(),
        classes: FxHashMap::default(),
        references: References::default(),
        hints: Hints::default(),
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
    for scope in &mut walker.scopes {
        if let Some(function) = scope.function
            && scope.kind == ScopeKind::Function
            && !scope.imports.is_empty()
        {
            walker
                .references
                .local_bindings
                .entry(function)
                .or_default()
                .extend(std::mem::take(&mut scope.imports));
        }
    }
    (walker.definitions, walker.references, walker.hints)
}

/// Walk a module once, and add what it defines and refers to.
struct Walker<'p, 'a> {
    /// The positions in the source of the module.
    positions: &'p SourcePositions<'a>,
    /// The definitions to add to.
    definitions: Vec<Definition>,
    /// The classes found so far, by their name in the module, each the first class of that name.
    classes: FxHashMap<String, Index>,
    /// The references to add to.
    references: References,
    /// The hints for frameworks to add to.
    hints: Hints,
    /// Every scope that was entered, by its index.
    scopes: Vec<Scope>,
    /// The scope that the node being walked is in.
    scope: usize,
    /// The scope that the names bound at the node belong to, which holds its definitions. It is the
    /// scope of the node, except in what a class or function evaluates where it is defined. The
    /// decorators, type parameters and base classes of a class bind in the class body. The decorators,
    /// default values and annotations of a function bind in a scope of their own that is never closed,
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
            Stmt::Expr(expr) => {
                // `__all__.extend([...])` and `__all__.append("...")` add to `__all__`.
                if let Some(names) = all_call_names(&expr.value) {
                    self.add_all_names(names);
                }
                walk_stmt(self, stmt);
            }
            Stmt::For(loop_) => {
                // A loop over a mapping reads its keys.
                let values = self.value_calls(&loop_.iter);
                self.add_whole_read(values, None);
                walk_stmt(self, stmt);
            }
            Stmt::Return(ast::StmtReturn {
                value: Some(value), ..
            }) => {
                self.add_returned(value);
                self.visit_passed(value);
            }
            Stmt::TypeAlias(alias) => {
                // Its value is evaluated when it is used, but what it reads is still read.
                self.define_target(&alias.name, false, None, None, None, None);
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
            self.define_target(target, false, None, None, None, None);
        }
        walk_with_item(self, item);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => {
                if name.ctx == ExprContext::Load {
                    self.add_read(&name.id, Vec::new(), None, false);
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
                self.visit_attributes(base, attributes, false);
            }
            Expr::Subscript(subscript) => {
                // A read of a string key (`value["name"]`).
                if subscript.ctx == ExprContext::Load {
                    self.add_key(&subscript.slice);
                    // `globals()["name"]` reads the name from the module.
                    if self.is_module_namespace(&subscript.value) {
                        self.add_module_read(&subscript.slice);
                    }
                }
                walk_expr(self, expr);
            }
            Expr::Call(call) => self.visit_call(call, None),
            Expr::Yield(ast::ExprYield {
                value: Some(value), ..
            }) => {
                self.add_returned(value);
                self.visit_passed(value);
            }
            // The elements of a container are passed on.
            Expr::List(ast::ExprList { elts, ctx, .. })
            | Expr::Tuple(ast::ExprTuple { elts, ctx, .. })
                if *ctx == ExprContext::Load =>
            {
                for element in elts {
                    self.visit_passed(element);
                }
            }
            Expr::Set(ast::ExprSet { elts, .. }) => {
                for element in elts {
                    self.visit_passed(element);
                }
            }
            Expr::Dict(dict) => {
                // `{**value}` reads every key of the value.
                for item in dict.iter().filter(|item| item.key.is_none()) {
                    let values = self.value_calls(&item.value);
                    self.add_whole_read(values, None);
                }
                for item in dict {
                    if let Some(key) = &item.key {
                        self.visit_expr(key);
                    }
                    self.visit_passed(&item.value);
                }
            }
            Expr::Compare(compare) => {
                // A read of a string key by a membership test (`"name" in value`).
                let operands = compare.operands.iter();
                let containers = compare.operands.iter().skip(1);
                for ((operand, operator), container) in
                    operands.zip(compare.ops.iter()).zip(containers)
                {
                    if matches!(operator, CmpOp::In | CmpOp::NotIn) {
                        self.add_key(operand);
                        // `"name" in globals()` reads the name from the module.
                        if self.is_module_namespace(container) {
                            self.add_module_read(operand);
                        }
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
                self.define_target(&named.target, false, None, None, None, None);
                self.visit_passed(&named.value);
                if let Expr::Name(name) = &*named.target {
                    let mut binder = self.binder;
                    while let Some(enclosing) = self.scopes[binder].enclosing {
                        binder = enclosing;
                    }
                    self.scopes[binder].bound.insert(name.id.to_string());
                }
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
                self.add_attributes(names.map(|keyword| keyword.attr.id.clone()));
            }
            _ => {}
        }
        walk_pattern(self, pattern);
    }
}
