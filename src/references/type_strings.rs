//! The names that the strings of type expressions read, as Python reads them when it evaluates the
//! type.

use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Operator};

use crate::definitions::{Unwrap, dotted_name};

use super::Walker;
use super::scopes::{Scope, ScopeKind};

impl Walker<'_, '_> {
    /// Add the reads of the names in the strings of a type expression that are types themselves, as
    /// Python reads them when it evaluates the type: a quoted annotation (`"Item"`), and a string in
    /// a subscript or a union of a type (`list["Item"]`, `int | "Item"`). The names are read in the
    /// scope of the type, as when they are not quoted. A string that is not a valid expression reads
    /// nothing.
    pub(super) fn visit_type_strings(&mut self, expr: &Expr) {
        let mut strings = Vec::new();
        type_strings(expr, &mut strings);
        for string in strings {
            let Ok(parsed) = ruff_python_parser::parse_expression(string.value.to_str().trim())
            else {
                continue;
            };
            // The positions of the parsed expression are in the string, so it binds in a scope of
            // its own that defines nothing.
            let binder = self.binder;
            self.binder = self.push_scope(Scope::new(ScopeKind::Function));
            self.visit_expr(parsed.expr());
            self.binder = binder;
            self.visit_type_strings(parsed.expr());
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
pub(super) fn type_arguments(call: &ast::ExprCall) -> Vec<&Expr> {
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
pub(super) fn typing_name(name: &str) -> Option<&str> {
    match name.rsplit_once('.') {
        None => Some(name),
        Some(("typing" | "typing_extensions", member)) => Some(member),
        Some(_) => None,
    }
}
