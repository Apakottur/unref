//! unittest runs the methods named `test*` of the subclasses of `unittest.TestCase`, with their
//! setup and teardown methods. Its `discover` command loads the files that match `test*.py`, and
//! calls their module setup and teardown, and their `load_tests` function.

use rustc_hash::FxHashSet;

use crate::definitions::{Definition, Kind};
use crate::models::fnmatch;

use super::{Context, Rules, Settings, file_name};

const UNITTEST_FILES: &str = "test*.py";
/// `unittest.TestCase` as `unittest` gives it out, and where it is defined.
const UNITTEST_TEST_CASES: [&str; 2] = ["unittest.TestCase", "unittest.case.TestCase"];
const UNITTEST_MODULE_FUNCTIONS: [&str; 3] = ["setUpModule", "tearDownModule", "load_tests"];
const UNITTEST_METHODS: [&str; 6] = [
    "setUp",
    "tearDown",
    "setUpClass",
    "tearDownClass",
    "asyncSetUp",
    "asyncTearDown",
];

/// The rules of unittest.
pub(super) struct Unittest {
    /// The classes that the test cases inherit from, by their dotted names.
    bases: FxHashSet<String>,
}

impl Rules for Unittest {
    fn new(_context: &Context, _settings: &Settings) -> Self {
        Self {
            bases: FxHashSet::default(),
        }
    }

    fn class(&mut self, context: &Context, module: usize, class: &Definition) {
        let name = &class.qualified_name;
        if is_test_case(context, module, name) {
            self.bases
                .extend(context.hierarchy.ancestors(module, name).iter().cloned());
        }
    }

    /// Check whether unittest finds and calls a definition by itself.
    fn uses(&self, context: &Context, definition: &Definition, module: usize) -> bool {
        // unittest finds the test cases and the functions of a module, not those in a function. The
        // methods of a test case in a function still count, since the function can return the class.
        if definition.kind == Kind::Class {
            return definition.owner.is_none()
                && is_test_case(context, module, &definition.qualified_name);
        }
        let name = definition.name.as_str();
        let Some(class) = context.class_name(definition, module) else {
            return definition.kind == Kind::Function
                && definition.owner.is_none()
                && UNITTEST_MODULE_FUNCTIONS.contains(&name)
                && fnmatch(file_name(context.path(module)), UNITTEST_FILES);
        };
        definition.kind == Kind::Method
            && (name.starts_with("test") || UNITTEST_METHODS.contains(&name))
            && (is_test_case(context, module, class)
                || self.bases.contains(&context.dotted_name(module, class)))
    }
}

/// Check whether a class of a module is a subclass of `unittest.TestCase`.
fn is_test_case(context: &Context, module: usize, name: &str) -> bool {
    context.inherits(module, name, &UNITTEST_TEST_CASES)
}
