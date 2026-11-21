//! The reads of names and of chains of attributes, and what calls refer to.

use ruff_python_ast::name::Name;
use ruff_python_ast::visitor::{Visitor, walk_expr};
use ruff_python_ast::{self as ast, Expr};

use super::scopes::ScopeKind;
use super::type_strings::type_arguments;
use super::{Read, Receiver, Walker};

/// The methods of a mapping that read a key, given as their first argument.
const KEY_METHODS: [&str; 3] = ["get", "pop", "setdefault"];
/// The functions that read an attribute named by their second argument, as `value.name` does.
const ATTRIBUTE_READERS: [&str; 2] = ["getattr", "hasattr"];

impl<'a> Walker<'_, '_> {
    /// Add a read of a name, or of a chain of attributes that starts with one.
    ///
    /// In a class body, a name that the body has bound so far is read from the class: it is a
    /// reference to the attributes, methods and classes of that name in the class, and to nothing else.
    /// Python reads a name of a class body that is not bound yet from the code around the class, so
    /// that read is passed on.
    ///
    /// `called` is the attribute read from the result of calling the chain, as in `Class().name`.
    pub(super) fn add_read(&mut self, root: &Name, attributes: Vec<Name>, called: Option<Name>) {
        let scope = &self.scopes[self.scope];
        if scope.kind == ScopeKind::Class
            && scope.bound.contains(root.as_str())
            && !scope.global_names.contains(root.as_str())
            && !scope.imports.contains_key(root.as_str())
        {
            let owner = self.owner;
            let members = scope.members.get(root.as_str()).into_iter().flatten();
            self.references
                .local_reads
                .extend(members.map(|&index| (owner, index)));
            self.add_attributes(attributes.into_iter().chain(called));
            return;
        }
        let read = Read {
            owner: self.owner,
            root: root.clone(),
            attributes,
            reads_object: !self.in_signature,
            called,
        };
        let scope = self.scope;
        self.scopes[scope].reads.push(read);
    }

    /// Add the read of a chain of attributes of an expression, such as `name` and `other` of
    /// `value.name.other`, and what the expression itself reads.
    pub(super) fn visit_attributes(&mut self, base: &'a Expr, mut attributes: Vec<Name>) {
        match base {
            // Whether the chain starts with an imported name is known when the scope ends.
            Expr::Name(name) => self.add_read(&name.id, attributes, None),
            Expr::Call(call) => {
                let first = attributes.remove(0);
                self.add_attributes(attributes);
                if let Some(class) = self.super_class(call) {
                    self.references
                        .members
                        .insert((self.owner, Receiver::Super(class), first));
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

    /// Check whether an expression is the namespace of the module: `globals()`, or `vars()` or
    /// `locals()` at the top level of the module, where they are the same.
    pub(super) fn is_module_namespace(&self, expr: &Expr) -> bool {
        let Expr::Call(call) = expr else {
            return false;
        };
        let Expr::Name(func) = &*call.func else {
            return false;
        };
        call.arguments.is_empty()
            && match func.id.as_str() {
                "globals" => true,
                "vars" | "locals" => self.scopes[self.scope].kind == ScopeKind::Module,
                _ => false,
            }
    }

    /// Add a read of a name of the module itself, by a string literal that is looked up in its
    /// namespace. The read skips the scopes in between, since the namespace holds no local variable.
    pub(super) fn add_module_read(&mut self, key: &Expr) {
        let Expr::StringLiteral(literal) = key else {
            return;
        };
        self.scopes[0].reads.push(Read {
            owner: self.owner,
            root: Name::new(literal.value.to_str()),
            attributes: Vec::new(),
            reads_object: true,
            called: None,
        });
    }

    /// Add the names that are read as attributes of an object whose class is not known.
    pub(super) fn add_attributes(&mut self, names: impl IntoIterator<Item = Name>) {
        let owner = self.owner;
        self.references
            .attributes
            .extend(names.into_iter().map(|name| (owner, name)));
    }

    /// Add what a call refers to.
    ///
    /// `called` is the attribute read from its result, as in `Class().name`. It is read from the
    /// class when what is called is a name, or a chain of attributes that starts with one.
    pub(super) fn visit_call(&mut self, call: &'a ast::ExprCall, called: Option<Name>) {
        // A read of a string key by a method of a mapping (`value.get("name")`).
        if let Expr::Attribute(func) = &*call.func
            && KEY_METHODS.contains(&func.attr.as_str())
            && let Some(first) = call.arguments.args.first()
        {
            self.add_key(first);
            // `globals().get("name")` reads the name from the module.
            if self.is_module_namespace(&func.value) {
                self.add_module_read(first);
            }
        }
        // `getattr(value, "name")` and `hasattr(value, "name")` read `value.name`.
        if let Expr::Name(func) = &*call.func
            && ATTRIBUTE_READERS.contains(&func.id.as_str())
            && call.arguments.keywords.is_empty()
            && let [value, Expr::StringLiteral(literal), rest @ ..] = &*call.arguments.args
        {
            self.add_read(&func.id, Vec::new(), None);
            let (base, mut attributes) = match value {
                Expr::Attribute(attribute) => attribute_chain(attribute),
                value => (value, Vec::new()),
            };
            attributes.push(Name::new(literal.value.to_str()));
            attributes.extend(called);
            self.visit_attributes(base, attributes);
            for argument in rest {
                self.visit_expr(argument);
            }
            return;
        }
        self.add_whole_reads(call);
        self.add_string_call(call);
        let (base, attributes) = match &*call.func {
            Expr::Attribute(func) => attribute_chain(func),
            func => (func, Vec::new()),
        };
        if let Expr::Name(name) = base {
            self.add_read(&name.id, attributes, called);
        } else {
            self.add_attributes(called);
            self.visit_expr(&call.func);
        }
        self.visit_arguments(&call.arguments);
        for argument in type_arguments(call) {
            self.visit_type_strings(argument);
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
}

/// Split a chain of attributes, such as `package.module.name`, into what it starts with and its
/// attributes in order.
pub(super) fn attribute_chain(attribute: &ast::ExprAttribute) -> (&Expr, Vec<Name>) {
    let mut attributes = vec![attribute.attr.id.clone()];
    let mut node = &*attribute.value;
    while let Expr::Attribute(inner) = node {
        attributes.push(inner.attr.id.clone());
        node = &inner.value;
    }
    attributes.reverse();
    (node, attributes)
}
