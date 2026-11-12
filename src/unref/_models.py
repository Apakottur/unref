import os
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

# The default of the `dummy-variable-rgx` option, the same as Ruff's.
DEFAULT_DUMMY_VARIABLE_RGX = r"^(_+|(_+[a-zA-Z0-9_]*[a-zA-Z0-9]+?))$"

# The test frameworks that are known, all of which are on by default.
FRAMEWORKS = ("pytest", "unittest")


@dataclass(frozen=True)
class Finding:
    """
    An unused definition, an ignore comment that ignores nothing, or a file that does not parse.

    Each finding has the code of its rule, a kebab-case name as ty gives its rules:

    - `unused-function`, `unused-class`, `unused-method`, `unused-variable` and `unused-attribute`
      for an unused definition.
    - `unused-ignore-comment` for an ignore comment that ignores nothing.
    - `invalid-syntax` for a file that does not parse.

    Attributes:
        path: The file that holds the definition.
        line: The line of the name of the definition in the file, starting at 1. For an ignore comment,
            the line of the comment. For a file that does not parse, the line of the error, or 1 if it
            is unknown.
        column: The column of the name of the definition, of the ignore comment, or of the error, in
            characters, starting at 1, as ruff and ty count them.
        code: The code of the rule, for example `unused-function`.
        name: The name of the definition, `unref: ignore` for an ignore comment, or the message of the
            error for a file that does not parse.
    """

    path: Path
    line: int
    column: int
    code: str
    name: str

    def __str__(self) -> str:
        return f"{self.path}:{self.line}:{self.column}: {self.code} '{self.name}'"


@dataclass(frozen=True)
class Override:
    """
    Options that apply only to some of the files of a project, on top of the options of the project.

    The options apply to the definitions that match every selector that is given: `paths`,
    `base_classes` and `class_decorators`.

    Attributes:
        paths: The files and directories that the options apply to.
        base_classes: The base classes, as dotted names such as `pydantic.BaseModel`, of the classes
            that the options apply to. The options apply to what is defined in a class that inherits
            from one of them, directly or not.
        class_decorators: The class decorators, as dotted names such as `dataclasses.dataclass`, of
            the classes that the options apply to. The options apply to what is defined in a class
            that has one of them.
        used_names: The names, as patterns, of the definitions that count as used.
        used_decorators: The decorators that make the definitions they decorate count as used.
    """

    paths: Sequence[str | os.PathLike[str]] = ()
    base_classes: Sequence[str] = ()
    class_decorators: Sequence[str] = ()
    used_names: Sequence[str] = ()
    used_decorators: Sequence[str] = ()
