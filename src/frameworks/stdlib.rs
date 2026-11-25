//! Classes of the standard library whose code calls methods of their subclasses that they do not
//! define, by a name that it makes when it runs or by the protocol that it follows.

use crate::definitions::{Definition, Kind};

use super::{Context, Rules, Settings};

/// `ast.NodeVisitor`, whose `visit` calls `visit_` and the name of the class of the node, and
/// `ast.NodeTransformer`, which inherits from it.
const NODE_VISITORS: [&str; 2] = ["ast.NodeVisitor", "ast.NodeTransformer"];
/// `http.server.BaseHTTPRequestHandler`, which calls `do_` and the method of the request, such as
/// `do_POST`.
const HTTP_HANDLERS: [&str; 1] = ["http.server.BaseHTTPRequestHandler"];
/// The finders of `importlib.abc`, whose `find_spec` the import system calls.
const FINDERS: [&str; 2] = [
    "importlib.abc.MetaPathFinder",
    "importlib.abc.PathEntryFinder",
];
/// The loaders of `importlib.abc`, whose `exec_module` the import system calls.
const LOADERS: [&str; 1] = ["importlib.abc.Loader"];

/// The rules of the standard library.
pub(super) struct Stdlib;

impl Rules for Stdlib {
    fn new(_context: &Context, _settings: &Settings) -> Self {
        Self
    }

    /// Check whether a class of the standard library calls a method of a subclass by its name.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        if definition.kind != Kind::Method {
            return false;
        }
        let Some(class) = context.class_name(definition, module) else {
            return false;
        };
        let name = definition.name.as_str();
        let bases: &[&str] = if name.starts_with("visit_") {
            &NODE_VISITORS
        } else if name.strip_prefix("do_").is_some_and(is_http_method) {
            &HTTP_HANDLERS
        } else if name == "find_spec" {
            &FINDERS
        } else if name == "exec_module" {
            &LOADERS
        } else {
            return false;
        };
        context.inherits(module, class, bases)
    }
}

/// Check whether a name is that of a method of HTTP as it can end the name of a method of Python:
/// uppercase, such as `POST` or `PATCH`.
fn is_http_method(name: &str) -> bool {
    name.starts_with(|char: char| char.is_ascii_uppercase())
        && name
            .chars()
            .all(|char| char.is_ascii_uppercase() || char.is_ascii_digit() || char == '_')
}
