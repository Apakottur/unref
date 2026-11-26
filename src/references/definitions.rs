//! The variables, attributes and functions that a module defines, and the names that its `__all__`
//! lists.

use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextSize};

use crate::definitions::{
    Definition, Kind, LOCALS, Unwrap, annotation_element_names, annotation_name, annotation_names,
    decorator_names, dotted_name,
};

use super::dynamic_imports::StringImport;
use super::scopes::{Scope, ScopeKind};
use super::type_strings::typing_name;
use super::{Binding, Index, Read, Walker};

impl<'a> Walker<'_, '_> {
    /// Add the names listed in `__all__`. Only the `__all__` of the module, at its top level or
    /// declared `global`, is its public interface.
    pub(super) fn add_all_names(&mut self, names: Vec<String>) {
        if self.binder != 0 && !self.scopes[self.binder].global_names.contains("__all__") {
            return;
        }
        self.references.has_all = true;
        for name in names {
            self.references.reads.insert(Read {
                owner: self.owner,
                root: name.into(),
                attributes: Vec::new(),
                reads_object: true,
                called: None,
                passed: false,
            });
        }
    }

    /// Add a variable of the module that is assigned a dotted name, or a union of them, such as `Base =
    /// logging.Handler` or `Message = Request | Response`, with each name as it is written.
    pub(super) fn add_alias(&mut self, target: &str, value: &Expr) {
        if self.scopes[self.binder].kind != ScopeKind::Module
            || matches!(value, Expr::StringLiteral(_))
        {
            return;
        }
        let names = annotation_names(value)
            .into_iter()
            .filter(|name| name != "None");
        self.references
            .aliases
            .entry(target.to_string())
            .or_default()
            .extend(names);
    }

    /// Add the variables and attributes that an assignment defines, the names that an assignment to
    /// `__all__` lists, and what the assignment refers to.
    pub(super) fn visit_assign(
        &mut self,
        targets: &'a [Expr],
        value: Option<&'a Expr>,
        annotation: Option<&'a Expr>,
        defines: bool,
    ) {
        let call = match (defines, value) {
            (true, Some(Expr::Call(call))) => dotted_name(&call.func, Unwrap::Call),
            _ => None,
        };
        if defines {
            if let ([Expr::Name(target)], Some(annotation)) = (targets, annotation)
                && self.scopes[self.binder].kind == ScopeKind::Function
            {
                let scope = &mut self.scopes[self.binder];
                scope
                    .annotations
                    .insert(target.id.to_string(), annotation_names(annotation));
                scope
                    .element_annotations
                    .insert(target.id.to_string(), annotation_element_names(annotation));
            }
            if let [target] = targets {
                self.add_attribute_types(target, value, annotation);
            }
            let annotation = annotation.map(annotation_name);
            let string = match value {
                Some(Expr::StringLiteral(literal)) => Some(literal.value.to_str().to_string()),
                _ => None,
            };
            // A call of the result of a call, as a decorator with arguments is applied by hand.
            let wrapped_by = match value {
                Some(Expr::Call(call)) => match &*call.func {
                    Expr::Call(decorator) => dotted_name(&decorator.func, Unwrap::Subscript),
                    _ => None,
                },
                _ => None,
            };
            for target in targets {
                self.define_target(
                    target,
                    false,
                    annotation.as_ref(),
                    call.as_ref(),
                    string.as_ref(),
                    wrapped_by.as_ref(),
                );
            }
            if let ([Expr::Name(target)], Some(value)) = (targets, value) {
                self.add_alias(target.id.as_str(), value);
            }
            // A variable of a function that is assigned one of several values, as `a if test else b`,
            // or another variable, is assigned what each of them is assigned the result of calling.
            if call.is_none()
                && let ([Expr::Name(target)], Some(value)) = (targets, value)
                && self.scopes[self.binder].kind == ScopeKind::Function
            {
                let calls = self.value_calls(value);
                if !calls.is_empty() {
                    let binder = self.binder;
                    self.scopes[binder]
                        .calls
                        .entry(target.id.to_string())
                        .or_default()
                        .extend(calls);
                }
            }
        }
        if targets.iter().any(is_all) {
            self.add_all_names(value.map_or_default(concatenated_string_literals));
        }
        // The value is evaluated before the targets are bound, so in a class body, `size = size + 1`
        // reads the name around the class.
        if let Some(value) = value {
            if defines {
                self.visit_passed(value);
            } else {
                self.visit_expr(value);
            }
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
        // A name assigned a module imported by a string stands for it, as if it were imported.
        if let ([Expr::Name(target)], Some(value)) = (targets, value)
            && defines
        {
            let binder = self.binder;
            match self.string_import(value) {
                Some(StringImport::Exact(module, level)) => {
                    let binding = Binding {
                        module: Some(module),
                        level,
                        name: None,
                    };
                    let name = target.id.to_string();
                    self.scopes[binder].string_imports.insert(name.clone());
                    self.bind_imports(std::iter::once((name, binding)).collect());
                }
                Some(StringImport::Pattern(pattern)) => {
                    self.scopes[binder]
                        .module_patterns
                        .insert(target.id.to_string(), pattern);
                }
                None => {}
            }
        }
        for target in targets {
            self.visit_expr(target);
            if self.binder == 0
                && let (Expr::Name(name), Some(call)) = (target, &call)
            {
                self.scopes[0]
                    .calls
                    .insert(name.id.to_string(), vec![call.clone()]);
            }
        }
    }

    /// Add the types of an attribute of a class, or of its instance in a method, that is annotated
    /// or assigned a value of a known type, as `value_types` gets them.
    fn add_attribute_types(
        &mut self,
        target: &Expr,
        value: Option<&Expr>,
        annotation: Option<&Expr>,
    ) {
        let scope = &self.scopes[self.binder];
        let (class, name) = match target {
            Expr::Name(name) if scope.kind == ScopeKind::Class => (self.binder, name.id.as_str()),
            Expr::Attribute(attribute)
                if matches!(&*attribute.value, Expr::Name(owner)
                    if scope.instance.as_deref() == Some(owner.id.as_str())
                        && !scope.bound.contains(owner.id.as_str())) =>
            {
                match scope.defined_in {
                    Some(class) if self.scopes[class].kind == ScopeKind::Class => {
                        (class, attribute.attr.as_str())
                    }
                    _ => return,
                }
            }
            _ => return,
        };
        let mut types = annotation.map_or_default(annotation_names);
        types.extend(value.map_or_default(|value| self.value_types(value)));
        if !types.is_empty() {
            self.scopes[class]
                .attribute_types
                .entry(name.to_string())
                .or_default()
                .extend(types);
        }
    }

    /// Add a function or a method.
    ///
    /// Returns the scope of its body, and its index in the definitions of the module.
    pub(super) fn define_function(&mut self, function: &ast::StmtFunctionDef) -> (Scope, Index) {
        let enclosing = &self.scopes[self.binder];
        let mut kind = Kind::Function;
        let mut scope = Scope::new(ScopeKind::Function);
        let mut class = None;
        let mut enclosing_function = None;
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
            scope.class = enclosing.class;
            class = enclosing.class;
        } else if !enclosing.global_names.contains(function.name.as_str()) {
            enclosing_function = enclosing.function;
        }
        let definition = Definition {
            decorators: decorator_names(&function.decorator_list),
            ..self.definition(
                kind,
                &function.name,
                function.name.start(),
                class,
                enclosing_function,
            )
        };
        let index = Index::new(self.definitions.len());
        scope.function = Some(index);
        self.definitions.push(definition);
        (scope, index)
    }

    /// Add the variables and attributes that an assignment target defines.
    pub(super) fn define_target(
        &mut self,
        target: &Expr,
        in_unpacking: bool,
        annotation: Option<&String>,
        call: Option<&String>,
        string: Option<&String>,
        wrapped_by: Option<&String>,
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
                    self.define_target(element, true, None, None, None, None);
                }
                return;
            }
            Expr::Starred(starred) => {
                self.define_target(&starred.value, true, None, None, None, None);
                return;
            }
            // A local variable, or an assignment into an object that is defined elsewhere.
            _ => return,
        };

        // A variable or attribute is found once, where it is first assigned.
        let class = scope.class;
        let in_class_body = scope.kind == ScopeKind::Class;
        let Some(defined_in) = scope.defined_in else {
            return;
        };
        let next = self.definitions.len();
        let index = *self.scopes[defined_in]
            .defined
            .entry(name.to_string())
            .or_insert(next);
        if in_class_body {
            // A method may have found the attribute first: the class body still binds it.
            let binder = self.binder;
            let members = self.scopes[binder]
                .members
                .entry(name.to_string())
                .or_default();
            if !members.contains(&index) {
                members.push(index);
            }
        }
        if index != next {
            return;
        }
        self.definitions.push(Definition {
            unpacked,
            annotation: annotation.cloned(),
            call: call.cloned(),
            string: string.cloned(),
            wrapped_by: wrapped_by.cloned(),
            ..self.definition(kind, name, position, class, None)
        });
    }

    /// Start a definition of a name at a position, in the function that the node is in, with the
    /// class that it is in and the function that it is in outside a class, and no decorators, base
    /// classes, annotation, call, string or wrapping decorator, not unpacked and not merged.
    pub(super) fn definition(
        &self,
        kind: Kind,
        name: &str,
        position: TextSize,
        class: Option<Index>,
        function: Option<Index>,
    ) -> Definition {
        let (line, column) = self.positions.line_column(position);
        let qualified_name = match (class, function) {
            (Some(class), _) => format!("{}.{name}", self.definitions[class.get()].qualified_name),
            (None, Some(function)) => {
                format!(
                    "{}{LOCALS}{name}",
                    self.definitions[function.get()].qualified_name
                )
            }
            (None, None) => name.to_string(),
        };
        Definition {
            line,
            column,
            kind,
            name: name.to_string(),
            qualified_name,
            decorators: Vec::new(),
            class,
            function,
            owner: self.owner,
            merged_into: None,
            bases: Vec::new(),
            other_bases: false,
            unpacked: false,
            annotation: None,
            call: None,
            string: None,
            wrapped_by: None,
        }
    }
}

/// Check whether an assignment target is the `__all__` variable.
fn is_all(target: &Expr) -> bool {
    matches!(target, Expr::Name(name) if name.id.as_str() == "__all__")
}

/// Get the string literals that `__all__.extend([...])` or `__all__.append("...")` adds to
/// `__all__`, or None if the expression is not such a call.
pub(super) fn all_call_names(expr: &Expr) -> Option<Vec<String>> {
    let Expr::Call(call) = expr else {
        return None;
    };
    let Expr::Attribute(attribute) = &*call.func else {
        return None;
    };
    if !is_all(&attribute.value) || !call.arguments.keywords.is_empty() {
        return None;
    }
    let [argument] = &*call.arguments.args else {
        return None;
    };
    match (attribute.attr.as_str(), argument) {
        ("extend", argument) => Some(string_literals(Some(argument))),
        ("append", Expr::StringLiteral(literal)) => Some(vec![literal.value.to_str().to_string()]),
        _ => None,
    }
}

/// Get the string literals of the list or tuple literals that an assignment to `__all__`
/// concatenates, as in `__all__ = base.__all__ + ["name"]`. Other operands, such as `__all__`
/// itself or the `__all__` of another module, have none.
fn concatenated_string_literals(value: &Expr) -> Vec<String> {
    match value {
        Expr::BinOp(binary) if binary.op == ast::Operator::Add => {
            let mut names = concatenated_string_literals(&binary.left);
            names.extend(concatenated_string_literals(&binary.right));
            names
        }
        _ => string_literals(Some(value)),
    }
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
