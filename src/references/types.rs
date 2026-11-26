//! What a Python module refers to, as the walk of its syntax tree finds it.

use ruff_python_ast::name::Name;
use rustc_hash::{FxHashMap, FxHashSet};

/// A name that a module imports: the module it is imported from, as it is written, the number of
/// leading dots, and the imported name. The name is None for `import module`, which binds the module
/// itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Binding {
    pub module: Option<String>,
    pub level: u32,
    pub name: Option<String>,
}

/// A definition of a module, by its index in the definitions of the module, in the order of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Index(pub u32);

impl Index {
    /// Make the index of a definition from its position in the definitions of its module, which has
    /// fewer than `u32::MAX` of them.
    pub(crate) fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("fewer definitions in a module than u32::MAX"))
    }

    /// The index, to look the definition up in the definitions of its module.
    pub(crate) fn get(self) -> usize {
        self.0 as usize
    }
}

/// The code that a reference or a definition is in: the function that it is in, or None for code that
/// runs when the module is imported.
pub(crate) type Owner = Option<Index>;

/// A read of a name, or of a chain of attributes that starts with one (`package.module.name`).
///
/// The names are kept as the syntax tree holds them, which copies a long name without allocating.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Read {
    pub owner: Owner,
    pub root: Name,
    /// The attributes, in order.
    pub attributes: Vec<Name>,
    /// Whether it reads the object, which a type hint of a parameter does not.
    pub reads_object: bool,
    /// The attribute read from the result of calling the chain, such as `name` in `Class().name`.
    pub called: Option<Name>,
    /// Whether the object is passed on: given to a function, assigned, put in a container, or returned.
    pub passed: bool,
}

/// A read of an imported name, or of a chain of attributes that starts with one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ImportedRead {
    pub owner: Owner,
    pub binding: Binding,
    pub attributes: Vec<Name>,
    pub reads_object: bool,
    pub called: Option<Name>,
    pub passed: bool,
}

/// The object that a member is read from, when the class that it is in is plain, with the class.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Receiver {
    /// The first parameter of a method of the class, as `self.name` or `cls.name`.
    Own(Index),
    /// `super()` in a method of the class, or `super(Class, self)`.
    Super(Index),
}

/// A name that is imported from another module (`from module import name`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Import {
    pub owner: Owner,
    /// The module, as it is written.
    pub module: Option<String>,
    /// The number of leading dots.
    pub level: u32,
    /// The name, `*` for a star import.
    pub name: String,
}

/// The modules that a string with values in it can name, as `import_module(f"plugins.{name}")` does:
/// those whose names hold its literal parts in order, with any text between them. The first part is
/// written as for an `import` statement, with the number of leading dots. There are always at least two
/// parts, the first and the last of which can be empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ModulePattern {
    pub parts: Vec<String>,
    pub level: u32,
}

impl ModulePattern {
    /// The pattern of any module.
    pub(crate) fn any() -> Self {
        Self {
            parts: vec![String::new(), String::new()],
            level: 0,
        }
    }

    /// Check whether the name of a module matches the pattern, once its first part is resolved.
    pub(crate) fn matches(parts: &[String], module: &str) -> bool {
        let [first, middle @ .., last] = parts else {
            return false;
        };
        let Some(mut rest) = module.strip_prefix(first.as_str()) else {
            return false;
        };
        for part in middle {
            let Some(index) = rest.find(part.as_str()) else {
                return false;
            };
            rest = &rest[index + part.len()..];
        }
        rest.ends_with(last.as_str())
    }
}

/// A value that is read as a whole, which reads every key of a TypedDict.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WholeRead {
    pub owner: Owner,
    /// The dotted name of what the value is the result of calling, as it is written, such as
    /// `RetryEvent` for `event = RetryEvent(...)`.
    pub value: String,
    /// The dotted name of the function that the value is passed to, as it is written, such as
    /// `json.dumps`, or None when it is read by `**`, a loop or a method of a mapping. It reads every
    /// key if it is from outside the project.
    pub reader: Option<String>,
    /// The import of the function that holds the read, if the reader starts with a name that it
    /// imports, such as `j` after `import json as j`.
    pub local_import: Option<Binding>,
}

/// What a Python module refers to, which the engine follows to find what is used.
///
/// Each reference is made from an owner: the function that it is in, or the module itself.
#[derive(Debug, Default)]
pub(crate) struct References {
    /// The reads of the names of the module itself, without the local variables and parameters. A name
    /// that is listed in `__all__` is read as a plain name. A read of a name that the module
    /// imports, at its top level or declared `global`, also reads its own name, without the object,
    /// such as a fallback definition in an `except ImportError`.
    pub reads: FxHashSet<Read>,
    /// The reads of the imported names, as for `reads`.
    pub imported_reads: FxHashSet<ImportedRead>,
    /// The names that are read as an attribute of something other than a name, such as a call
    /// (`call().name`) or a local variable, and those that a class pattern matches by keyword.
    pub attributes: FxHashSet<(Owner, Name)>,
    /// The names that are read as a member of the class of a method, through its first parameter or
    /// `super()`.
    pub members: FxHashSet<(Owner, Receiver, Name)>,
    /// The string keys that are read from a mapping: `value["name"]`, `value.get("name")`,
    /// `"name" in value`, or a mapping pattern (`case {"name": ...}`).
    pub keys: FxHashSet<(Owner, String)>,
    /// The values that are read as a whole, or passed to a function: `{**value}`, `f(**value)`,
    /// `for key in value`, `value.items()` or `f(value)`, when the value is the result of a call, or a
    /// local variable that is assigned one.
    pub whole_reads: FxHashSet<WholeRead>,
    /// The names that are read as attributes of a module imported by a string with values in it, as
    /// `module.name` after `module = import_module(f"plugins.{name}")`, each with the modules that it
    /// can be read from.
    pub pattern_reads: FxHashSet<(Owner, ModulePattern, Name)>,
    /// The modules whose names match a pattern that are passed on, as `return module` after
    /// `module = import_module(f"plugins.{name}")`.
    pub passed_patterns: FxHashSet<(Owner, ModulePattern)>,
    /// The names that are imported from another module. A star import has the name `*`.
    pub imports: FxHashSet<Import>,
    /// Whether the module defines `__all__`.
    pub has_all: bool,
    /// The names that the module imports at its top level, each with what it stands for.
    pub bindings: FxHashMap<String, Binding>,
    /// The names that the functions of the module import, by the index of the function in the
    /// definitions of the module, each with what it stands for.
    pub local_bindings: FxHashMap<Index, FxHashMap<String, Binding>>,
    /// The functions and classes that are local variables of a function, by their index in the
    /// definitions of the module. A read of their name elsewhere is not a reference to them.
    pub local_definitions: FxHashSet<usize>,
    /// The reads of the functions and classes that are local variables of a function, and of the
    /// members of a class in its class body, each with the index of the definition in the definitions
    /// of the module.
    pub local_reads: FxHashSet<(Owner, usize)>,
    /// The reads of the objects of the functions and classes that are local variables of a function,
    /// each with the index of the definition in the definitions of the module and the attributes read
    /// from it, in order, as in `Color.parse` for an enum.
    pub local_objects: FxHashSet<(Owner, usize, Vec<Name>)>,
}

/// What a Python module reveals about its values that only the rules of frameworks read, such as the
/// variables that hold an application of a web framework, or the modules that Django names by a
/// string. The walk finds them with the references, and the engine leaves them to the frameworks.
#[derive(Debug, Default)]
pub(crate) struct Hints {
    /// The local variables of the functions that are assigned the result of a call.
    pub local_calls: LocalCalls,
    /// The local variables of the functions that are assigned some module whose name matches a
    /// pattern, as `module = import_module(f"plugins.{name}")`, with the pattern.
    pub local_patterns: FxHashMap<(Index, String), ModulePattern>,
    /// The annotated parameters of the functions, by the function and the name of the parameter, each
    /// with the dotted name of its annotation as it is written, without its type arguments, such as
    /// `Flask` for `app: Flask`.
    pub annotated_parameters: FxHashMap<(Index, String), String>,
    /// The parameters of the functions, by the function and the name of the parameter.
    pub parameters: FxHashSet<(Index, String)>,
    /// The functions that return or yield the result of a call, or a local variable that is assigned
    /// one, each with the dotted names of what they call as they are written, such as `Flask` for
    /// `return Flask(__name__)`.
    pub returned_calls: FxHashMap<Index, Vec<String>>,
    /// The calls of the functions and methods whose string arguments frameworks read, each by the
    /// dotted name of what it calls as it is written, such as `os.environ.setdefault`, and its leading
    /// positional arguments that are string literals.
    pub string_calls: FxHashSet<(String, Vec<String>)>,
    /// The calls of `config.from_object`, whose argument Flask reads the uppercase attributes of, each
    /// by the function that it is in, the dotted name of what it calls as it is written, such as
    /// `app.config.from_object`, and its argument.
    pub config_objects: FxHashSet<(Owner, String, ConfigObject)>,
}

/// What `config.from_object` of Flask is given: a dotted name as it is written, such as `Config` or
/// `__name__`, or a string that names a module or a class in it, such as `"config.Production"`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum ConfigObject {
    Name(String),
    String(String),
}

/// The local variables of each function of a module that are assigned the result of a call, by the
/// function and the name of the variable, each with the dotted names of what they call as they are
/// written, such as `FastAPI` for `app = FastAPI()`.
pub(crate) type LocalCalls = FxHashMap<(Index, String), Vec<String>>;
