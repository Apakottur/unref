import os
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from types import MappingProxyType

# The default of the `dummy-variable-rgx` option, the same as Ruff's.
DEFAULT_DUMMY_VARIABLE_RGX = r"^(_+|(_+[a-zA-Z0-9_]*[a-zA-Z0-9]+?))$"


class Framework(StrEnum):
    """
    A framework whose built-in rules can apply, by its name in the `frameworks` option.
    """

    PYTEST = "pytest"
    UNITTEST = "unittest"
    PYDANTIC = "pydantic"
    SQLALCHEMY = "sqlalchemy"


# Whether each framework is enabled by default.
FRAMEWORKS: Mapping[Framework, bool] = MappingProxyType(
    {Framework.PYTEST: True, Framework.UNITTEST: True, Framework.PYDANTIC: True, Framework.SQLALCHEMY: True}
)

# The codes of the findings, which an ignore comment can name.
CODES = frozenset(
    {
        "unused-function",
        "unused-class",
        "unused-method",
        "unused-variable",
        "unused-attribute",
        "unused-ignore-comment",
        "invalid-ignore-comment",
        "ignore-comment-unknown-rule",
        "invalid-syntax",
    }
)


@dataclass(frozen=True)
class Finding:
    """
    An unused definition, a problem with an ignore comment, or a file that does not parse.

    `str(finding)` gives the line that the command prints, `path:line:column: code 'name'`.

    Attributes:
        path: The file of the finding.
        line: The line of the finding, starting at 1.
        column: The column of the finding, starting at 1, as ruff and ty count it.
        code: The code of the rule, such as `unused-function`, see
            [Rules](https://unref.readthedocs.io/en/latest/rules/).
        name: The name of the unused definition, or what is wrong.
        transitive: Whether the unused definition is reached only from other unused definitions, which
            are reported too: it is called or read by them, or defined in them. The command shows it
            only with `show-transitive`. Always false for the other findings.
    """

    path: Path
    line: int
    column: int
    code: str
    name: str
    transitive: bool = False

    def __str__(self) -> str:
        return f"{self.path}:{self.line}:{self.column}: {self.code} '{self.name}'"


@dataclass(frozen=True)
class Override:
    """
    Options that apply only to some of the definitions of a project, on top of those of the project.

    An override needs at least one selector, and applies to what matches all of its selectors. See
    [Overrides](https://unref.readthedocs.io/en/latest/configuration/#overrides).

    Attributes:
        paths: Selects the definitions in these files and directories.
        base_classes: Selects the definitions in subclasses of these classes, such as `ast.NodeVisitor`.
        class_decorators: Selects the definitions in classes with these decorators, such as
            `dataclasses.dataclass`.
        used_names: Names, as patterns, of the selected definitions that count as used.
        used_decorators: Decorators that make the selected definitions they decorate count as used.
    """

    paths: Sequence[str | os.PathLike[str]] = ()
    base_classes: Sequence[str] = ()
    class_decorators: Sequence[str] = ()
    used_names: Sequence[str] = ()
    used_decorators: Sequence[str] = ()
