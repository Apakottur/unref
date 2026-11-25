//! What a module reveals about its values beyond the names it reads: the string keys it reads and the
//! values it reads whole, which are references, and the calls whose string arguments frameworks read
//! and what its functions return, which are hints for frameworks.

use ruff_python_ast::{self as ast, Expr};

use crate::definitions::{Unwrap, dotted_name};

use super::{Walker, WholeRead};

/// The methods of a mapping that read every key.
const WHOLE_METHODS: [&str; 3] = ["items", "keys", "values"];
/// A hint for frameworks: the last names of the functions and methods whose string arguments a
/// framework reads, which the walk keeps the calls of. They are Django's `include`, which names a
/// module of URLs, and `os.environ.setdefault`, which can set `DJANGO_SETTINGS_MODULE`. The rules of
/// Django, in `frameworks/django.rs`, check what each call resolves to.
const FRAMEWORK_STRING_CALLS: [&str; 2] = ["include", "setdefault"];

impl Walker<'_, '_> {
    /// Add a call of a function or method whose string arguments frameworks read, with its leading
    /// positional arguments that are string literals.
    pub(super) fn add_string_call(&mut self, call: &ast::ExprCall) {
        // The last name is checked first, as most calls are of other functions.
        let mut func = &*call.func;
        while let Expr::Call(inner) = func {
            func = &inner.func;
        }
        let last = match func {
            Expr::Name(name) => name.id.as_str(),
            Expr::Attribute(attribute) => attribute.attr.as_str(),
            _ => return,
        };
        if !FRAMEWORK_STRING_CALLS.contains(&last) {
            return;
        }
        let Some(called) = dotted_name(&call.func, Unwrap::Call) else {
            return;
        };
        let arguments = call
            .arguments
            .args
            .iter()
            .map_while(|argument| match argument {
                Expr::StringLiteral(literal) => Some(literal.value.to_str().to_string()),
                _ => None,
            })
            .collect();
        self.hints.string_calls.insert((called, arguments));
    }

    /// Add the values that a call reads as a whole, `f(**value)` and `value.items()`, and those that
    /// it passes to a function, which reads them whole if it is from outside the project.
    pub(super) fn add_whole_reads(&mut self, call: &ast::ExprCall) {
        if let Expr::Attribute(func) = &*call.func
            && WHOLE_METHODS.contains(&func.attr.as_str())
            && call.arguments.is_empty()
        {
            let values = self.value_calls(&func.value);
            self.add_whole_read(values, None);
        }
        for keyword in &call.arguments.keywords {
            if keyword.arg.is_none() {
                let values = self.value_calls(&keyword.value);
                self.add_whole_read(values, None);
            }
        }
        // The name of the function is found only for a call that passes such a value, as few do.
        let mut reader: Option<Option<String>> = None;
        let passed = call.arguments.args.iter().chain(
            call.arguments
                .keywords
                .iter()
                .filter(|keyword| keyword.arg.is_some())
                .map(|keyword| &keyword.value),
        );
        for argument in passed {
            let values = self.value_calls(argument);
            if values.is_empty() {
                continue;
            }
            if let Some(reader) =
                reader.get_or_insert_with(|| dotted_name(&call.func, Unwrap::Call))
            {
                self.add_whole_read(values, Some(reader));
            }
        }
    }

    /// Add a read of a value as a whole, given by the dotted names of what it is the result of calling.
    pub(super) fn add_whole_read(&mut self, values: Vec<String>, reader: Option<&str>) {
        let owner = self.owner;
        let local_import = reader
            .and_then(|reader| self.local_import(reader.split('.').next().unwrap_or(reader)))
            .cloned();
        self.references
            .whole_reads
            .extend(values.into_iter().map(|value| WholeRead {
                owner,
                value,
                reader: reader.map(str::to_string),
                local_import: local_import.clone(),
            }));
    }

    /// Add a value that the function the walker is in returns or yields, if it is the result of a call.
    pub(super) fn add_returned(&mut self, value: &Expr) {
        let calls = self.value_calls(value);
        if let Some(function) = self.owner
            && !calls.is_empty()
        {
            self.hints
                .returned_calls
                .entry(function)
                .or_default()
                .extend(calls);
        }
    }

    /// Get the dotted names of what a value is the result of calling, as they are written, if it is a
    /// call, a variable that is assigned one in the current scope, or `a if test else b` of them.
    pub(super) fn value_calls(&self, value: &Expr) -> Vec<String> {
        match value {
            Expr::Call(call) => dotted_name(&call.func, Unwrap::Call).into_iter().collect(),
            Expr::Name(name) => self.variable_calls(name.id.as_str()),
            Expr::If(choice) => {
                let mut calls = self.value_calls(&choice.body);
                calls.extend(self.value_calls(&choice.orelse));
                calls
            }
            _ => Vec::new(),
        }
    }

    /// Add a read of a key, if it is a string literal.
    pub(super) fn add_key(&mut self, key: &Expr) {
        if let Expr::StringLiteral(literal) = key {
            self.references
                .keys
                .insert((self.owner, literal.value.to_str().to_string()));
        }
    }
}
