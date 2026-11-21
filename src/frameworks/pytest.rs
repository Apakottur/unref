//! pytest collects the files that match its `python_files` option, `test_*.py` and `*_test.py`, and
//! reads `conftest.py`. In such a file, it runs the functions that match `python_functions`,
//! `test*`, and the methods that match it in the classes that match `python_classes`, `Test*`. It
//! calls the setup and teardown functions of a module, class or method, the hooks of its plugins,
//! `pytest_*`, and the fixtures, wherever they are.

use std::path::Path;

use crate::definitions::{Definition, Kind};
use crate::models::fnmatch;

use super::{FrameworkRules, file_name};

const PYTEST_FILES: [&str; 2] = ["test_*.py", "*_test.py"];
const PYTEST_CONFTEST: &str = "conftest.py";
/// `pytest.fixture` as `pytest` gives it out, and where it is defined, and the same for the fixtures of
/// pytest-asyncio.
pub(super) const PYTEST_FIXTURES: [&str; 4] = [
    "pytest.fixture",
    "_pytest.fixtures.fixture",
    "pytest_asyncio.fixture",
    "pytest_asyncio.plugin.fixture",
];
/// The functions of a test module that pytest calls, besides the tests.
const PYTEST_MODULE_FUNCTIONS: [&str; 6] = [
    "setup_module",
    "teardown_module",
    "setUpModule",
    "tearDownModule",
    "setup_function",
    "teardown_function",
];
/// The methods of a test class that pytest calls, besides the tests.
const PYTEST_CLASS_METHODS: [&str; 4] = [
    "setup_class",
    "teardown_class",
    "setup_method",
    "teardown_method",
];
/// The variables that pytest reads from a test module or `conftest.py`, of which it reads
/// `pytestmark` from a test class too.
const PYTEST_MODULE_VARIABLES: [&str; 2] = ["pytestmark", "pytest_plugins"];
/// The variables that pytest reads from `conftest.py` only.
const PYTEST_CONFTEST_VARIABLES: [&str; 2] = ["collect_ignore", "collect_ignore_glob"];

impl FrameworkRules<'_> {
    /// Check whether pytest finds and calls a definition by itself.
    pub(super) fn pytest_uses(&self, definition: &Definition, module: usize) -> bool {
        let (kind, name, path) = (definition.kind, definition.name.as_str(), self.path(module));
        // The hooks of a plugin, which can be any module or object that pytest registers, and the
        // fixtures.
        if (kind.is_function() && name.starts_with("pytest_"))
            || self.is_decorated(definition, module, &PYTEST_FIXTURES)
        {
            return true;
        }
        // pytest collects what is at the top level of a module or in a class there, not what is in
        // a function.
        if definition.owner.is_some() {
            return false;
        }

        let Some(class) = self.class_name(definition, module) else {
            if file_name(path) == PYTEST_CONFTEST {
                return kind == Kind::Variable
                    && (PYTEST_MODULE_VARIABLES.contains(&name)
                        || PYTEST_CONFTEST_VARIABLES.contains(&name));
            }
            return is_pytest_file(path)
                && ((kind == Kind::Function
                    && (name.starts_with("test") || PYTEST_MODULE_FUNCTIONS.contains(&name)))
                    || (kind == Kind::Class && name.starts_with("Test"))
                    || (kind == Kind::Variable && PYTEST_MODULE_VARIABLES.contains(&name)));
        };

        if kind == Kind::Class {
            return is_pytest_class(path, &definition.qualified_name);
        }
        ((kind == Kind::Method
            && (name.starts_with("test") || PYTEST_CLASS_METHODS.contains(&name)))
            || (kind == Kind::Attribute && name == "pytestmark"))
            && (is_pytest_class(path, class)
                || self.pytest_bases.contains(&self.dotted_name(module, class)))
    }
}

/// Check whether pytest collects the tests of a file.
fn is_pytest_file(path: &Path) -> bool {
    PYTEST_FILES
        .iter()
        .any(|pattern| fnmatch(file_name(path), pattern))
}

/// Check whether pytest collects a class of a module: a class of a test file whose name starts with
/// `Test`, at the top level or in such a class.
pub(super) fn is_pytest_class(path: &Path, name: &str) -> bool {
    is_pytest_file(path) && name.split('.').all(|part| part.starts_with("Test"))
}
