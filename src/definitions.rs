//! What a Python module defines.

use ruff_python_ast::{self as ast, Decorator, Expr};

use crate::models::Rule;
use crate::references::{Index, Owner};

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
    /// The rule of the finding of a definition of this kind that is unused, such as `unused-function`.
    pub(crate) fn code(self) -> Rule {
        match self {
            Kind::Function => Rule::UnusedFunction,
            Kind::Class => Rule::UnusedClass,
            Kind::Method => Rule::UnusedMethod,
            Kind::Variable => Rule::UnusedVariable,
            Kind::Attribute => Rule::UnusedAttribute,
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
    /// The line of the name of the definition in the file, starting at 1.
    pub line: u32,
    /// The column of the name of the definition in its line, in characters, starting at 1.
    pub column: u32,
    /// What is defined.
    pub kind: Kind,
    /// The name of the definition.
    pub name: String,
    /// The name of the definition in its module, such as `Outer.Inner` for a nested class, and
    /// `function.<locals>.Local` for a class that is defined in a function.
    pub qualified_name: String,
    /// The dotted names of the decorators of the definition, such as `app.command`, without their
    /// arguments. A decorator that is not a dotted name is left out.
    pub decorators: Vec<String>,
    /// The class that the definition is in, or None if it is not in a class. Classes of one name in
    /// the module, such as the two of an `if` and its `else`, are one class, the first of them, with
    /// the base classes, decorators and members of each.
    pub class: Option<Index>,
    /// For a function or a class that is defined in a function and not in a class of it, that
    /// function, or None otherwise. One that the function declares `global` is defined in the module.
    pub function: Option<Index>,
    /// The function that the definition is in, or None if it is not in a function. A class in a
    /// function is in it, and so are the methods of the class.
    pub owner: Owner,
    /// For a class that has the name in the module of a class before it, that first class, which it
    /// is one class with, or None otherwise.
    pub merged_into: Option<Index>,
    /// For a class, the dotted names of its base classes as they are written, without their type
    /// arguments. A base class that is not a dotted name is left out.
    pub bases: Vec<String>,
    /// For a class, whether one of its base classes is not a dotted name, such as a call that makes
    /// one, so that what it inherits from is not known.
    pub other_bases: bool,
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
    /// For a variable or an attribute that is first assigned a string literal, its value, such as
    /// `project.urls` for `ROOT_URLCONF = "project.urls"`.
    pub string: Option<String>,
    /// For a variable or an attribute that is first assigned a function that a decorator with
    /// arguments wraps by hand, the dotted name of the decorator as it is written, such as
    /// `field_validator` for `field_validator("name")(check)`.
    pub wrapped_by: Option<String>,
}

/// What is between the name of a function and the name of what is defined in it, as in the
/// qualified name that Python gives it.
pub(crate) const LOCALS: &str = ".<locals>.";

/// The kind of expression that the outer end of a dotted name is unwrapped from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unwrap {
    /// A decorator or what a call calls, such as `app.command` for `@app.command("name")`.
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

/// Get the dotted names of an annotation, one for each side of a union: `Model | None`,
/// `Optional[Model]` or `Union[Model, Other]`, as written. A string annotation is parsed.
pub(crate) fn annotation_names(annotation: &Expr) -> Vec<String> {
    if let Expr::StringLiteral(literal) = annotation {
        return match ruff_python_parser::parse_expression(literal.value.to_str().trim()) {
            Ok(parsed) => annotation_names(parsed.expr()),
            Err(_) => Vec::new(),
        };
    }
    match annotation {
        Expr::BinOp(union) if union.op == ast::Operator::BitOr => {
            let mut names = annotation_names(&union.left);
            names.extend(annotation_names(&union.right));
            names
        }
        Expr::Subscript(subscript)
            if dotted_name(&subscript.value, Unwrap::Subscript).is_some_and(|name| {
                matches!(name.rsplit('.').next(), Some("Optional" | "Union"))
            }) =>
        {
            match &*subscript.slice {
                Expr::Tuple(tuple) => tuple.iter().flat_map(annotation_names).collect(),
                other => annotation_names(other),
            }
        }
        _ => dotted_name(annotation, Unwrap::Subscript)
            .into_iter()
            .collect(),
    }
}

/// Get the dotted name of an expression, such as a decorator, a base class, an annotation or what a
/// call calls.
///
/// Only the outer call or subscript is unwrapped, so `@app().route` and `@registry["x"].route` are not
/// dotted names.
///
/// Returns None if the expression is not a dotted name, for example `@decorators[0]` or `make_base()`.
pub(crate) fn dotted_name(expression: &Expr, unwrap: Unwrap) -> Option<String> {
    let expression = match (expression, unwrap) {
        (Expr::Call(call), Unwrap::Call) => &*call.func,
        (Expr::Subscript(subscript), Unwrap::Subscript) => &*subscript.value,
        _ => expression,
    };
    let mut name = String::new();
    push_dotted_name(expression, &mut name).then_some(name)
}

/// Add the dotted name of an expression to the end of a string, which is built in place since many
/// calls and assignments are checked.
///
/// Returns false if the expression is not a dotted name.
fn push_dotted_name(expression: &Expr, name: &mut String) -> bool {
    match expression {
        Expr::Name(root) => {
            name.push_str(root.id.as_str());
            true
        }
        Expr::Attribute(attribute) => {
            if !push_dotted_name(&attribute.value, name) {
                return false;
            }
            name.push('.');
            name.push_str(attribute.attr.as_str());
            true
        }
        _ => false,
    }
}
