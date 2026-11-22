//! The modules that a module imports by a string, as `importlib.import_module(name)` does.

use ruff_python_ast::{self as ast, Expr, Operator};

use crate::definitions::{Unwrap, dotted_name};
use crate::resolver::join;

use super::{Binding, ModulePattern, Walker};

/// What a call that imports a module by a string gives.
pub(super) enum StringImport {
    /// The module, written as for an `import` statement, with the number of leading dots.
    Exact(String, u32),
    /// Some module whose name matches the literal parts of the string, as for
    /// `import_module(f"plugins.{name}")`, or any module for a string with no literal part.
    Pattern(ModulePattern),
}

/// A part of a string that is built by an f-string or by `+`.
enum Part<'a> {
    Literal(String),
    Value(&'a Expr),
}

impl Walker<'_, '_> {
    /// Find the module that a call imports by a string, if it is `importlib.import_module(name)` or
    /// `__import__(name)`.
    ///
    /// A string relative to a package, as in `import_module(".name", __package__)`, is relative to
    /// the module that holds the call, or to the package given as a string. So is a string that
    /// starts with `__name__` or `__package__`, as in `f"{__name__}.name"`, and one that starts with
    /// the `__name__` of an imported module is relative to that module. A string with values in it
    /// gives a module whose name matches its literal parts, in order. `__import__` gives the top
    /// package of the module, or the module itself with a `fromlist`.
    pub(super) fn string_import(&self, value: &Expr) -> Option<StringImport> {
        let Expr::Call(call) = value else {
            return None;
        };
        let arguments = &call.arguments;
        let (name, package) = match dotted_name(&call.func, Unwrap::Call)?.as_str() {
            "importlib.import_module" | "import_module" => (
                arguments.find_argument_value("name", 0)?,
                arguments.find_argument_value("package", 1),
            ),
            "__import__" => {
                let name = arguments.find_argument_value("name", 0)?;
                if arguments.find_argument_value("fromlist", 3).is_none() {
                    return Some(match name {
                        Expr::StringLiteral(literal) => {
                            let module = literal.value.to_str();
                            let top = module.split('.').next().unwrap_or(module);
                            StringImport::Exact(top.to_string(), 0)
                        }
                        _ => StringImport::Pattern(ModulePattern::any()),
                    });
                }
                (name, None)
            }
            _ => return None,
        };
        let any = || Some(StringImport::Pattern(ModulePattern::any()));
        let mut parts = string_parts(name).into_iter().peekable();
        // The module that a leading `__name__` stands for, as written with its leading dots. The
        // `__name__` of a module that is not imported is any name.
        let start = match parts.peek() {
            // Only a package has modules in it, so `__name__` is a package here.
            Some(Part::Value(Expr::Name(name)))
                if matches!(name.id.as_str(), "__name__" | "__package__") =>
            {
                Some((String::new(), 1))
            }
            Some(Part::Value(Expr::Attribute(attribute)))
                if attribute.attr.as_str() == "__name__" =>
            {
                dotted_name(&attribute.value, Unwrap::Call)
                    .and_then(|module| self.import_of(&module))
                    .map(|binding| {
                        let module = binding.module.as_deref().unwrap_or_default();
                        let name = binding.name.as_deref().unwrap_or_default();
                        (join(module, name), binding.level)
                    })
            }
            _ => None,
        };
        if start.is_some() {
            parts.next();
        }
        let start = start.unwrap_or_default();
        // The literal parts, between which the values stand.
        let mut literals = vec![String::new()];
        for part in parts {
            match part {
                Part::Literal(literal) => literals.last_mut()?.push_str(&literal),
                Part::Value(_) => literals.push(String::new()),
            }
        }
        let first = &literals[0];
        let (first, level) = match start {
            (module, 0) if module.is_empty() => {
                let dots = first.chars().take_while(|&char| char == '.').count();
                let rest = first[dots..].to_string();
                match (dots, package) {
                    (0, _) => (rest, 0),
                    (_, Some(Expr::Name(package)))
                        if matches!(package.id.as_str(), "__package__" | "__name__") =>
                    {
                        (rest, u32::try_from(dots).ok()?)
                    }
                    (1, Some(Expr::StringLiteral(package))) => {
                        (join(package.value.to_str(), &rest), 0)
                    }
                    // Relative to a package that is only known when the code runs.
                    _ => return any(),
                }
            }
            // The rest goes on from the module, as in `f"{__name__}.name"`.
            (module, level) => match first.strip_prefix('.') {
                Some(rest) => (join(&module, rest), level),
                None if first.is_empty() && literals.len() > 1 => (module, level),
                None => return any(),
            },
        };
        literals[0] = first;
        Some(if literals.len() == 1 {
            StringImport::Exact(literals.swap_remove(0), level)
        } else {
            StringImport::Pattern(ModulePattern {
                parts: literals,
                level,
            })
        })
    }

    /// Get what a name stands for if it is imported, in the function that the walker is in, one
    /// around it, or the module.
    fn import_of(&self, name: &str) -> Option<&Binding> {
        self.local_import(name)
            .or_else(|| self.scopes[0].imports.get(name))
    }
}

/// Split a string into its parts, if it is built by an f-string, `+` or `%`, in order.
fn string_parts(value: &Expr) -> Vec<Part<'_>> {
    match value {
        Expr::StringLiteral(literal) => vec![Part::Literal(literal.value.to_str().to_string())],
        Expr::FString(string) => string
            .value
            .elements()
            .map(|element| match element {
                ast::InterpolatedStringElement::Literal(literal) => {
                    Part::Literal(literal.to_string())
                }
                ast::InterpolatedStringElement::Interpolation(interpolation) => {
                    Part::Value(&interpolation.expression)
                }
            })
            .collect(),
        Expr::BinOp(binary) if binary.op == Operator::Add => {
            let mut parts = string_parts(&binary.left);
            parts.extend(string_parts(&binary.right));
            parts
        }
        Expr::BinOp(binary) if binary.op == Operator::Mod => match &*binary.left {
            Expr::StringLiteral(left) => {
                let mut segments = left.value.to_str().split('%');
                let mut parts = vec![Part::Literal(
                    segments.next().unwrap_or_default().to_string(),
                )];
                for segment in segments {
                    // Each value replaces a conversion, such as `%s` or `%(name)s`.
                    let named = segment
                        .strip_prefix('(')
                        .and_then(|segment| Some(segment.split_once(')')?.1));
                    let rest = named
                        .unwrap_or(segment)
                        .trim_start_matches(|char: char| !char.is_ascii_alphabetic())
                        .get(1..)
                        .unwrap_or_default();
                    parts.extend([Part::Value(&binary.right), Part::Literal(rest.to_string())]);
                }
                parts
            }
            _ => vec![Part::Value(value)],
        },
        _ => vec![Part::Value(value)],
    }
}
