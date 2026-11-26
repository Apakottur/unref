//! The functions, lambdas, classes and comprehensions of a module, each walked in a scope of its own.

use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Parameters, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::definitions::{
    Definition, Kind, Unwrap, annotation_element_names, annotation_name, annotation_names,
    decorator_names, dotted_name,
};

use super::scopes::{Scope, ScopeKind, Target};
use super::{Index, Walker, WholeRead};

/// The body of a function or lambda.
#[derive(Clone, Copy)]
pub(super) enum FunctionBody<'a> {
    Statements(&'a [Stmt]),
    Lambda(&'a Expr),
}

impl<'a> Walker<'_, '_> {
    /// Add what a comprehension refers to.
    ///
    /// The variables of a comprehension are local to it. Its first iterable is evaluated in the scope
    /// that the comprehension is in, so it sees the names of a class body, and the rest does not.
    pub(super) fn visit_comprehension(
        &mut self,
        elements: &[&'a Expr],
        generators: &'a [ast::Comprehension],
    ) {
        let Some(first) = generators.first() else {
            return;
        };
        // The first iterable is read outside the comprehension.
        self.add_whole_read(&first.iter, None);
        self.visit_expr(&first.iter);
        let (scope, binder) = (self.scope, self.binder);
        let target = self.reads_target(scope);
        let mut inner = Scope::new(ScopeKind::Function);
        // A walrus in a comprehension binds outside it. Python disallows it in a class body.
        if self.scopes[binder].kind != ScopeKind::Class {
            inner.enclosing = Some(binder);
        }
        inner.comprehension_parent = Some(target);
        self.scope = self.push_scope(inner);
        self.binder = self.scope;
        for (index, generator) in generators.iter().enumerate() {
            if index > 0 {
                // A comprehension over a mapping reads its keys in the current scope,
                // after the earlier iteration targets have been bound.
                self.add_whole_read(&generator.iter, None);
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
        self.close(inner, &local_names, Target::Scope(target));
    }

    pub(super) fn visit_function_def(&mut self, function: &'a ast::StmtFunctionDef) {
        let (body_scope, index) = self.define_function(function);
        // The bounds and defaults of the type parameters are read where the function is defined.
        if let Some(type_params) = &function.type_params {
            self.visit_type_params(type_params);
        }
        self.visit_function(
            Some(&function.parameters),
            &function.decorator_list,
            function.returns.as_deref(),
            Some((body_scope, index)),
            FunctionBody::Statements(&function.body),
        );
        // The name is bound once the function is defined, after its decorators are evaluated.
        self.bind_definition(function.name.as_str(), index.get());
    }

    /// Add what a function or lambda refers to, with the scope of its body and the function, or None
    /// for a lambda.
    pub(super) fn visit_function(
        &mut self,
        parameters: Option<&'a Parameters>,
        decorators: &'a [ast::Decorator],
        returns: Option<&'a Expr>,
        defined: Option<(Scope, Index)>,
        body: FunctionBody<'a>,
    ) {
        let mut names: Vec<&str> = Vec::new();
        let mut defaults: Vec<&Expr> = Vec::new();
        let mut annotations: Vec<&Expr> = Vec::new();
        let mut annotated: Vec<(String, Vec<String>)> = Vec::new();
        let mut element_annotated: Vec<(String, Vec<String>)> = Vec::new();
        if let Some(parameters) = parameters {
            for parameter in parameters {
                names.push(parameter.name().as_str());
                if let Some((_, function)) = &defined {
                    self.hints
                        .parameters
                        .insert((*function, parameter.name().to_string()));
                }
                if let Some(default) = parameter.default() {
                    defaults.push(default);
                }
                if let Some(annotation) = parameter.annotation() {
                    annotations.push(annotation);
                    annotated.push((parameter.name().to_string(), annotation_names(annotation)));
                    element_annotated.push((
                        parameter.name().to_string(),
                        annotation_element_names(annotation),
                    ));
                    if let Some((_, function)) = &defined {
                        let name = annotation_name(annotation);
                        if !name.is_empty() {
                            self.hints
                                .annotated_parameters
                                .insert((*function, parameter.name().to_string()), name);
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
        // A type hint of a parameter or the return reads no value of the object that it names, except
        // that a decorator can read the signature of the function, as FastAPI and Typer read the
        // members of an enum that annotates a parameter.
        for annotation in annotations {
            self.in_signature = decorators.is_empty();
            self.visit_expr(annotation);
            self.visit_type_strings(annotation);
        }
        self.in_signature = true;
        if let Some(annotation) = returns {
            self.visit_expr(annotation);
            self.visit_type_strings(annotation);
        }
        self.in_signature = false;

        // The body runs only when the function is called. A lambda has no name, and is part of the code
        // that it is in.
        let (mut body_scope, function) = match defined {
            Some((body_scope, function)) => (body_scope, Some(function)),
            None => (Scope::new(ScopeKind::Function), None),
        };
        body_scope.annotations.extend(annotated);
        body_scope.element_annotations.extend(element_annotated);
        let (scope, owner) = (self.scope, self.owner);
        body_scope.function_parent = Some(self.reads_target(scope));
        self.scope = self.push_scope(body_scope);
        self.binder = self.scope;
        match body {
            FunctionBody::Lambda(expr) => self.visit_expr(expr),
            FunctionBody::Statements(statements) => {
                self.owner = function;
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
        if let Some(function) = function {
            let calls = calls
                .into_iter()
                .filter(|(name, _)| local_names.contains(name));
            self.hints
                .local_calls
                .extend(calls.map(|(name, called)| ((function, name), called)));
            let patterns = inner_scope
                .module_patterns
                .iter()
                .filter(|(name, _)| local_names.contains(*name));
            self.hints.local_patterns.extend(
                patterns.map(|(name, pattern)| ((function, name.clone()), pattern.clone())),
            );
        }
        local_names.extend(names.iter().map(|name| (*name).to_string()));
        let target = self.reads_target(scope);
        self.close(inner, &local_names, Target::Scope(target));
    }

    /// Add a class, and what it defines and refers to.
    ///
    /// The body of a class is a scope whose names are the attributes, methods and classes of the class.
    /// The functions in it do not see them.
    pub(super) fn visit_class(&mut self, class: &'a ast::StmtClassDef) {
        let binder = &self.scopes[self.binder];
        let (enclosing_class, function) = if binder.kind == ScopeKind::Class {
            (binder.class, None)
        } else if binder.global_names.contains(class.name.as_str()) {
            // A class that a function declares `global` is a class of the module.
            (None, None)
        } else {
            (None, binder.function)
        };
        let written_bases = class
            .arguments
            .as_deref()
            .map_or_default(|arguments| &*arguments.args);
        let bases: Vec<String> = written_bases
            .iter()
            .filter_map(|base| dotted_name(base, Unwrap::Subscript))
            .collect();
        let other_bases = bases.len() < written_bases.len();
        let index = self.definitions.len();
        let mut definition = Definition {
            decorators: decorator_names(&class.decorator_list),
            bases,
            other_bases,
            ..self.definition(
                Kind::Class,
                &class.name,
                class.name.start(),
                enclosing_class,
                function,
            )
        };
        // A class of the name of one before it is one class with it.
        let first = *self
            .classes
            .entry(definition.qualified_name.clone())
            .or_insert(Index::new(index));
        if first.get() != index {
            definition.merged_into = Some(first);
        }
        let mut class_scope = Scope::new(ScopeKind::Class);
        class_scope.class = Some(first);
        class_scope.outer = Some(self.reads_target(self.scope));
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
        // The methods read the attributes of the instance as a whole by the types that the class
        // gives them anywhere in its body.
        let class_scope = &mut self.scopes[inner];
        let types = std::mem::take(&mut class_scope.attribute_types);
        for (attribute, read) in std::mem::take(&mut class_scope.attribute_reads) {
            for value in types.get(&attribute).into_iter().flatten() {
                self.references.whole_reads.insert(WholeRead {
                    value: value.clone(),
                    ..read.clone()
                });
            }
        }

        // The names that the class body imports are resolved in it, and its other reads are of names
        // that it had not bound yet, so they go on.
        let imported = self.scopes[inner].imports.keys().cloned().collect();
        let target = self.reads_target(inner);
        self.close(inner, &imported, Target::Scope(target));
        // The name is bound once the class is defined.
        self.bind_definition(class.name.as_str(), index);
    }
}
