"""
The definitions that test frameworks find and call by themselves, which count as used.

The rules follow what the frameworks do with their default settings:

- pytest collects the files that match its `python_files` option, `test_*.py` and `*_test.py`, and
  reads `conftest.py`. In such a file, it runs the functions that match `python_functions`, `test*`,
  and the methods that match it in the classes that match `python_classes`, `Test*`. It calls the
  setup and teardown functions of a module, class or method, the hooks of its plugins, `pytest_*`,
  and the fixtures, wherever they are.
- unittest runs the methods named `test*` of the subclasses of `unittest.TestCase`, with their setup
  and teardown methods. Its `discover` command loads the files that match `test*.py`, and calls their
  module setup and teardown, and their `load_tests` function.

Both find the methods of a test class through its base classes as well, so a base class of a test
class, wherever it is, holds tests.
"""

from collections.abc import Collection
from fnmatch import fnmatchcase
from pathlib import Path

from unref._classes import ClassHierarchy
from unref._definitions import Definition
from unref._modules import ModuleName

_PYTEST_FILES = ("test_*.py", "*_test.py")
_PYTEST_CONFTEST = "conftest.py"
# `pytest.fixture` as `pytest` gives it out, and where it is defined.
_PYTEST_FIXTURES = frozenset({"pytest.fixture", "_pytest.fixtures.fixture"})
# The functions of a test module that pytest calls, besides the tests.
_PYTEST_MODULE_FUNCTIONS = frozenset(
    {
        "setup_module",
        "teardown_module",
        "setUpModule",
        "tearDownModule",
        "setup_function",
        "teardown_function",
    }
)
# The methods of a test class that pytest calls, besides the tests.
_PYTEST_CLASS_METHODS = frozenset({"setup_class", "teardown_class", "setup_method", "teardown_method"})
# The variables that pytest reads from a test module or `conftest.py`, and from a test class.
_PYTEST_MODULE_VARIABLES = frozenset({"pytestmark", "pytest_plugins"})
_PYTEST_CONFTEST_VARIABLES = frozenset({"pytestmark", "pytest_plugins", "collect_ignore", "collect_ignore_glob"})
_PYTEST_CLASS_VARIABLES = frozenset({"pytestmark"})

_UNITTEST_FILES = "test*.py"
# `unittest.TestCase` as `unittest` gives it out, and where it is defined.
_UNITTEST_TEST_CASES = frozenset({"unittest.TestCase", "unittest.case.TestCase"})
_UNITTEST_MODULE_FUNCTIONS = frozenset({"setUpModule", "tearDownModule", "load_tests"})
_UNITTEST_METHODS = frozenset({"setUp", "tearDown", "setUpClass", "tearDownClass", "asyncSetUp", "asyncTearDown"})


class FrameworkRules:
    """
    The rules of the test frameworks of a project.
    """

    def __init__(
        self,
        frameworks: Collection[str],
        *,
        definitions: dict[Path, list[Definition]],
        names: dict[Path, ModuleName],
        hierarchy: ClassHierarchy,
    ) -> None:
        """
        Args:
            frameworks: The test frameworks whose rules apply.
            definitions: The definitions of each file of the project.
            names: The module name of each file.
            hierarchy: The hierarchy of the classes of the project.
        """
        self._pytest = "pytest" in frameworks
        self._unittest = "unittest" in frameworks
        self._names = names
        self._hierarchy = hierarchy

        # The classes that the test classes inherit from, by their dotted names.
        self._pytest_bases: set[str] = set()
        self._unittest_bases: set[str] = set()
        for path, file_definitions in definitions.items():
            for definition in file_definitions:
                if definition.kind != "class":
                    continue
                if self._pytest and _is_pytest_class(path, definition.qualified_name):
                    self._pytest_bases |= hierarchy.ancestors(path, definition.qualified_name)
                if self._unittest and self._is_test_case(path, definition.qualified_name):
                    self._unittest_bases |= hierarchy.ancestors(path, definition.qualified_name)

    def uses(self, definition: Definition, path: Path) -> bool:
        """
        Check whether a test framework finds and calls a definition by itself.

        Args:
            definition: The definition.
            path: The file of the definition.

        Returns:
            Whether one of the frameworks uses the definition.
        """
        return (self._pytest and self._pytest_uses(definition, path)) or (
            self._unittest and self._unittest_uses(definition, path)
        )

    def _pytest_uses(self, definition: Definition, path: Path) -> bool:
        """
        Check whether pytest finds and calls a definition by itself.
        """
        kind, name = definition.kind, definition.name
        # The hooks of a plugin, which can be any module or object that pytest registers, and the
        # fixtures.
        if (kind in ("function", "method") and name.startswith("pytest_")) or any(
            not self._hierarchy.resolve(path, decorator).isdisjoint(_PYTEST_FIXTURES)
            for decorator in definition.decorators
        ):
            return True

        if definition.owner is None:
            if path.name == _PYTEST_CONFTEST:
                return kind == "variable" and name in _PYTEST_CONFTEST_VARIABLES
            return _is_pytest_file(path) and (
                (kind == "function" and (name.startswith("test") or name in _PYTEST_MODULE_FUNCTIONS))
                or (kind == "class" and name.startswith("Test"))
                or (kind == "variable" and name in _PYTEST_MODULE_VARIABLES)
            )

        if kind == "class":
            return _is_pytest_class(path, definition.qualified_name)
        return (
            (kind == "method" and (name.startswith("test") or name in _PYTEST_CLASS_METHODS))
            or (kind == "attribute" and name in _PYTEST_CLASS_VARIABLES)
        ) and (
            _is_pytest_class(path, definition.owner) or self._dotted_name(path, definition.owner) in self._pytest_bases
        )

    def _unittest_uses(self, definition: Definition, path: Path) -> bool:
        """
        Check whether unittest finds and calls a definition by itself.
        """
        if definition.kind == "class":
            return self._is_test_case(path, definition.qualified_name)
        if definition.owner is None:
            return (
                definition.kind == "function"
                and definition.name in _UNITTEST_MODULE_FUNCTIONS
                and fnmatchcase(path.name, _UNITTEST_FILES)
            )
        return (
            definition.kind == "method"
            and (definition.name.startswith("test") or definition.name in _UNITTEST_METHODS)
            and (
                self._is_test_case(path, definition.owner)
                or self._dotted_name(path, definition.owner) in self._unittest_bases
            )
        )

    def _is_test_case(self, path: Path, name: str) -> bool:
        """
        Check whether a class of a module is a subclass of `unittest.TestCase`.
        """
        return not self._hierarchy.ancestors(path, name).isdisjoint(_UNITTEST_TEST_CASES)

    def _dotted_name(self, path: Path, name: str) -> str:
        """
        Get the dotted name of a class of a module, as it is among the base classes of a class.
        """
        return f"{self._names[path].name}.{name}"


def _is_pytest_file(path: Path) -> bool:
    """
    Check whether pytest collects the tests of a file.
    """
    return any(fnmatchcase(path.name, pattern) for pattern in _PYTEST_FILES)


def _is_pytest_class(path: Path, name: str) -> bool:
    """
    Check whether pytest collects a class of a module: a class of a test file whose name starts with
    `Test`, at the top level or in such a class.
    """
    return _is_pytest_file(path) and all(part.startswith("Test") for part in name.split("."))
