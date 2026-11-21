//! unittest runs the methods named `test*` of the subclasses of `unittest.TestCase`, with their
//! setup and teardown methods. Its `discover` command loads the files that match `test*.py`, and
//! calls their module setup and teardown, and their `load_tests` function.

use crate::definitions::{Definition, Kind};
use crate::models::fnmatch;

use super::{FrameworkRules, file_name};

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

impl FrameworkRules<'_> {
    /// Check whether unittest finds and calls a definition by itself.
    pub(super) fn unittest_uses(&self, definition: &Definition, module: usize) -> bool {
        // unittest finds the test cases and the functions of a module, not those in a function. The
        // methods of a test case in a function still count, since the function can return the class.
        if definition.kind == Kind::Class {
            return definition.owner.is_none()
                && self.is_test_case(module, &definition.qualified_name);
        }
        let name = definition.name.as_str();
        let Some(class) = self.class_name(definition, module) else {
            return definition.kind == Kind::Function
                && definition.owner.is_none()
                && UNITTEST_MODULE_FUNCTIONS.contains(&name)
                && fnmatch(file_name(self.path(module)), UNITTEST_FILES);
        };
        definition.kind == Kind::Method
            && (name.starts_with("test") || UNITTEST_METHODS.contains(&name))
            && (self.is_test_case(module, class)
                || self
                    .unittest_bases
                    .contains(&self.dotted_name(module, class)))
    }

    /// Check whether a class of a module is a subclass of `unittest.TestCase`.
    pub(super) fn is_test_case(&self, module: usize, name: &str) -> bool {
        self.inherits(module, name, &UNITTEST_TEST_CASES)
    }
}
