//! What a Python module defines.

use std::path::Path;
use std::sync::Arc;

use ruff_python_ast::{Decorator, Expr};

/// What is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    Function,
    Class,
    Method,
    Variable,
    Attribute,
}

impl Kind {
    /// The code of the finding of a definition of this kind that is unused, such as `unused-function`.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Kind::Function => "unused-function",
            Kind::Class => "unused-class",
            Kind::Method => "unused-method",
            Kind::Variable => "unused-variable",
            Kind::Attribute => "unused-attribute",
        }
    }

    /// Whether the definition has code of its own that runs when it is called.
    pub(crate) fn is_function(self) -> bool {
        matches!(self, Kind::Function | Kind::Method)
    }
}

/// Something that a Python module defines.
#[derive(Debug, Clone)]
pub(crate) struct Definition {
    /// The file that holds the definition.
    pub path: Arc<Path>,
    /// The line of the name of the definition in the file, starting at 1.
    pub line: u32,
    /// The column of the name of the definition in its line, in characters, starting at 1.
    pub column: u32,
    /// What is defined.
    pub kind: Kind,
    /// The name of the definition.
    pub name: String,
    /// The dotted names of the decorators of the definition, such as `app.command`, without their
    /// arguments. A decorator that is not a dotted name is left out.
    pub decorators: Vec<String>,
    /// The class that the definition is in, by its name in the module (`Outer.Inner` for a nested
    /// class), or None if it is not in a class.
    pub owner: Option<String>,
    /// The function that the definition is in, by the line of its name, or None if it is not in a
    /// function. A class in a function is in it, and so are the methods of the class.
    pub function: Option<u32>,
    /// For a class, the dotted names of its base classes as they are written, without their type
    /// arguments. A base class that is not a dotted name is left out.
    pub bases: Vec<String>,
    /// For a variable or a class attribute, whether it is first assigned by its name in an unpacking,
    /// such as `second` in `first, second = pair()`, where Python needs a name to bind.
    pub unpacked: bool,
    /// For a variable or an attribute that is first assigned with an annotation, the dotted name of
    /// the annotation as it is written, without its type arguments, such as `ClassVar` for
    /// `ClassVar[int]`. An empty string if the annotation is not a dotted name, such as `int | None`,
    /// and None if there is no annotation.
    pub annotation: Option<String>,
    /// For a variable or an attribute that is first assigned the result of a call, the dotted name of
    /// what it calls as it is written, such as `mapped_column` for `mapped_column(Integer)`, or None
    /// if the value is not a call of a dotted name.
    pub call: Option<String>,
}

impl Definition {
    /// The name of the definition in its module, such as `Outer.Inner` for a nested class.
    pub(crate) fn qualified_name(&self) -> String {
        match &self.owner {
            None => self.name.clone(),
            Some(owner) => format!("{owner}.{}", self.name),
        }
    }
}

/// The kind of expression that a dotted name is unwrapped from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unwrap {
    /// A decorator, such as `app.command` for `@app.command("name")`.
    Call,
    /// A base class or an annotation, such as `pydantic.BaseModel` for `pydantic.BaseModel[int]`.
    Subscript,
}

/// Get the dotted names of the decorators of a class or function.
pub(crate) fn decorator_names(decorators: &[Decorator]) -> Vec<String> {
    decorators
        .iter()
        .filter_map(|decorator| dotted_name(&decorator.expression, Unwrap::Call))
        .collect()
}

/// Get the dotted name of an annotation, without its type arguments, such as `ClassVar` for
/// `ClassVar[int]`, also when the annotation is quoted. An empty string if it is not a dotted name.
pub(crate) fn annotation_name(annotation: &Expr) -> String {
    if let Expr::StringLiteral(literal) = annotation {
        return match ruff_python_parser::parse_expression(literal.value.to_str().trim()) {
            Ok(parsed) => dotted_name(parsed.expr(), Unwrap::Subscript).unwrap_or_default(),
            Err(_) => String::new(),
        };
    }
    dotted_name(annotation, Unwrap::Subscript).unwrap_or_default()
}

/// Get the dotted name of a decorator or a base class.
///
/// Returns None if the expression is not a dotted name, for example `@decorators[0]` or `make_base()`.
pub(crate) fn dotted_name(expression: &Expr, unwrap: Unwrap) -> Option<String> {
    match expression {
        Expr::Call(call) if unwrap == Unwrap::Call => dotted_name(&call.func, unwrap),
        Expr::Subscript(subscript) if unwrap == Unwrap::Subscript => {
            dotted_name(&subscript.value, unwrap)
        }
        Expr::Name(name) => Some(name.id.to_string()),
        Expr::Attribute(attribute) => {
            let prefix = dotted_name(&attribute.value, unwrap)?;
            Some(format!("{prefix}.{}", attribute.attr.as_str()))
        }
        _ => None,
    }
}
