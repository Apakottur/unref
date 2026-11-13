"""
The definitions that frameworks find, call or read by themselves, which count as used.

The rules follow what the frameworks do with their default settings:

- pytest collects the files that match its `python_files` option, `test_*.py` and `*_test.py`, and
  reads `conftest.py`. In such a file, it runs the functions that match `python_functions`, `test*`,
  and the methods that match it in the classes that match `python_classes`, `Test*`. It calls the
  setup and teardown functions of a module, class or method, the hooks of its plugins, `pytest_*`,
  and the fixtures, wherever they are.
- unittest runs the methods named `test*` of the subclasses of `unittest.TestCase`, with their setup
  and teardown methods. Its `discover` command loads the files that match `test*.py`, and calls their
  module setup and teardown, and their `load_tests` function.

- Pydantic reads the fields of the subclasses of `pydantic.BaseModel`, the names they annotate other
  than class variables and private attributes, and their `model_config`, or their `Config` class in
  Pydantic 1. It calls the methods that its validator and serializer decorators register, and reads
  the properties that `computed_field` registers.
- SQLAlchemy maps the attributes of its declarative models: the subclasses of
  `sqlalchemy.orm.DeclarativeBase`, `DeclarativeBaseNoMeta` or `MappedAsDataclass`, of a class that
  `declarative_base()` makes or that `as_declarative()` decorates, and the classes, such as mixins,
  that they inherit from. An attribute is mapped when its annotation is `Mapped`, or its value is a
  call of a construct such as `mapped_column`, `Column` or `relationship`. SQLAlchemy calls the
  functions that `event.listens_for`, `validates`, `reconstructor` and `declared_attr` register.

pytest and unittest find the methods of a test class through its base classes as well, so a base class
of a test class, wherever it is, holds tests.
"""

from collections.abc import Collection
from fnmatch import fnmatchcase
from pathlib import Path

from unref._classes import ClassHierarchy
from unref._definitions import Definition
from unref._modules import ModuleName

_PYTEST_FILES = ("test_*.py", "*_test.py")
_PYTEST_CONFTEST = "conftest.py"
# `pytest.fixture` as `pytest` gives it out, and where it is defined, and the same for the fixtures of
# pytest-asyncio.
_PYTEST_FIXTURES = frozenset(
    {"pytest.fixture", "_pytest.fixtures.fixture", "pytest_asyncio.fixture", "pytest_asyncio.plugin.fixture"}
)
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

# `pydantic.BaseModel` as `pydantic` gives it out, and where it is defined, also in the Pydantic 1 API of
# `pydantic.v1`. Packages such as `pydantic_settings` import it from where it is defined.
_PYDANTIC_MODELS = frozenset(
    {"pydantic.BaseModel", "pydantic.main.BaseModel", "pydantic.v1.BaseModel", "pydantic.v1.main.BaseModel"}
)
# The decorators that register a method with Pydantic, as `pydantic` gives them out, and where they are
# defined, also in the Pydantic 1 API of `pydantic.v1`.
_PYDANTIC_DECORATORS = frozenset(
    {
        "pydantic.field_validator",
        "pydantic.functional_validators.field_validator",
        "pydantic.model_validator",
        "pydantic.functional_validators.model_validator",
        "pydantic.field_serializer",
        "pydantic.functional_serializers.field_serializer",
        "pydantic.model_serializer",
        "pydantic.functional_serializers.model_serializer",
        "pydantic.computed_field",
        "pydantic.fields.computed_field",
        "pydantic.validator",
        "pydantic.deprecated.class_validators.validator",
        "pydantic.root_validator",
        "pydantic.deprecated.class_validators.root_validator",
        "pydantic.v1.validator",
        "pydantic.v1.class_validators.validator",
        "pydantic.v1.root_validator",
        "pydantic.v1.class_validators.root_validator",
    }
)
_PYDANTIC_CONFIG = "model_config"
# The configuration class of a model in Pydantic 1.
_PYDANTIC_V1_CONFIG = "Config"
_CLASS_VARIABLES = frozenset({"typing.ClassVar", "typing_extensions.ClassVar"})

# SQLAlchemy's names are matched anywhere under `sqlalchemy`, by their end, as it gives most of them out
# from several modules, such as `sqlalchemy.Column` and `sqlalchemy.schema.Column`.
# The base classes of declarative models.
_SQLALCHEMY_BASES = frozenset({"DeclarativeBase", "DeclarativeBaseNoMeta", "MappedAsDataclass"})
# The function that makes a base class of declarative models, and the decorator that makes one.
_SQLALCHEMY_BASE_FUNCTIONS = frozenset({"declarative_base"})
_SQLALCHEMY_BASE_DECORATORS = frozenset({"as_declarative"})
# The annotations and constructs of mapped attributes.
_SQLALCHEMY_ANNOTATIONS = frozenset({"Mapped", "WriteOnlyMapped", "DynamicMapped"})
_SQLALCHEMY_CONSTRUCTS = frozenset(
    {
        "Column",
        "mapped_column",
        "relationship",
        "column_property",
        "composite",
        "synonym",
        "deferred",
        "query_expression",
        "dynamic_loader",
    }
)
# The decorators that register a function with SQLAlchemy.
_SQLALCHEMY_DECORATORS = frozenset(
    {
        "listens_for",
        "validates",
        "reconstructor",
        "declared_attr",
        "declared_attr.directive",
        "declared_attr.cascading",
    }
)


class FrameworkRules:
    """
    The rules of the frameworks of a project.
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
            frameworks: The frameworks whose rules apply.
            definitions: The definitions of each file of the project.
            names: The module name of each file.
            hierarchy: The hierarchy of the classes of the project.
        """
        self._pytest = "pytest" in frameworks
        self._unittest = "unittest" in frameworks
        self._pydantic = "pydantic" in frameworks
        self._sqlalchemy = "sqlalchemy" in frameworks
        self._names = names
        self._hierarchy = hierarchy

        # The declarative models of SQLAlchemy, and the classes that they inherit from, such as mixins,
        # by their dotted names.
        self._mapped_classes = self._sqlalchemy_classes(definitions) if self._sqlalchemy else set()

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
        Check whether a framework finds, calls or reads a definition by itself.

        Args:
            definition: The definition.
            path: The file of the definition.

        Returns:
            Whether one of the frameworks uses the definition.
        """
        return (
            (self._pytest and self._pytest_uses(definition, path))
            or (self._unittest and self._unittest_uses(definition, path))
            or (self._pydantic and self._pydantic_uses(definition, path))
            or (self._sqlalchemy and self._sqlalchemy_uses(definition, path))
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

    def _pydantic_uses(self, definition: Definition, path: Path) -> bool:
        """
        Check whether Pydantic calls or reads a definition by itself.
        """
        owner = definition.owner
        if owner is None:
            return False
        if definition.kind == "method":
            # A decorator registers the method with Pydantic, in a model or a Pydantic dataclass.
            return any(
                not self._hierarchy.resolve(path, decorator).isdisjoint(_PYDANTIC_DECORATORS)
                for decorator in definition.decorators
            )
        if definition.kind == "class":
            return definition.name == _PYDANTIC_V1_CONFIG and self._is_model(path, owner)

        # An attribute.
        model, _, last = owner.rpartition(".")
        if last == _PYDANTIC_V1_CONFIG and model and self._is_model(path, model):
            return True
        return self._is_model(path, owner) and (
            definition.name == _PYDANTIC_CONFIG
            or (
                # A field: an annotated name that is not a private attribute or a class variable.
                definition.annotation is not None
                and not definition.name.startswith("_")
                and self._hierarchy.resolve(path, definition.annotation).isdisjoint(_CLASS_VARIABLES)
            )
        )

    def _is_model(self, path: Path, name: str) -> bool:
        """
        Check whether a class of a module is a subclass of `pydantic.BaseModel`.
        """
        return not self._hierarchy.ancestors(path, name).isdisjoint(_PYDANTIC_MODELS)

    def _sqlalchemy_uses(self, definition: Definition, path: Path) -> bool:
        """
        Check whether SQLAlchemy maps or calls a definition by itself.
        """
        if definition.kind in ("function", "method"):
            return any(
                _is_sqlalchemy(self._hierarchy.resolve(path, decorator), _SQLALCHEMY_DECORATORS)
                for decorator in definition.decorators
            )
        if definition.kind != "attribute" or definition.owner is None:
            return False
        return self._dotted_name(path, definition.owner) in self._mapped_classes and (
            (
                definition.annotation is not None
                and _is_sqlalchemy(self._hierarchy.resolve(path, definition.annotation), _SQLALCHEMY_ANNOTATIONS)
            )
            or (
                definition.call is not None
                and _is_sqlalchemy(self._hierarchy.resolve(path, definition.call), _SQLALCHEMY_CONSTRUCTS)
            )
        )

    def _sqlalchemy_classes(self, definitions: dict[Path, list[Definition]]) -> set[str]:
        """
        Get the dotted names of the declarative models of SQLAlchemy, and of the classes that they
        inherit from.
        """
        # The base classes that the project makes with SQLAlchemy's function or decorator.
        made: set[str] = set()
        classes: list[tuple[Path, str]] = []
        for path, file_definitions in definitions.items():
            for definition in file_definitions:
                name = definition.qualified_name
                if definition.kind == "class":
                    classes.append((path, name))
                    if _is_sqlalchemy(self._hierarchy.decorators(path, name), _SQLALCHEMY_BASE_DECORATORS):
                        made.add(self._dotted_name(path, name))
                elif (
                    definition.kind == "variable"
                    and definition.call is not None
                    and _is_sqlalchemy(self._hierarchy.resolve(path, definition.call), _SQLALCHEMY_BASE_FUNCTIONS)
                ):
                    made.add(self._dotted_name(path, name))

        # A variable that a class inherits from in its own module is among its base classes by its name
        # alone.
        models = set(made)
        for path, name in classes:
            ancestors = self._hierarchy.ancestors(path, name)
            local = {self._dotted_name(path, ancestor) for ancestor in ancestors if "." not in ancestor}
            if _is_sqlalchemy(ancestors, _SQLALCHEMY_BASES) or not made.isdisjoint(ancestors | local):
                models.add(self._dotted_name(path, name))

        # A model can inherit from a model of another module, whose base class is known only there.
        mapped = set(models)
        for path, name in classes:
            ancestors = self._hierarchy.ancestors(path, name)
            if self._dotted_name(path, name) in models or not ancestors.isdisjoint(models):
                mapped |= {self._dotted_name(path, name), *ancestors}
        return mapped

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


def _is_sqlalchemy(names: frozenset[str], ends: frozenset[str]) -> bool:
    """
    Check whether one of the dotted names of something is a name of SQLAlchemy that ends with one of
    the given names, such as `sqlalchemy.orm.mapped_column` for `mapped_column`.
    """
    return any(name.startswith("sqlalchemy.") and name.endswith(tuple(f".{end}" for end in ends)) for name in names)
