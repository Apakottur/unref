import ast
from dataclasses import dataclass
from pathlib import Path
from typing import Literal


@dataclass(frozen=True)
class Definition:
    """
    Something that a Python module defines.

    Attributes:
        path: The file that holds the definition.
        line: The line of the definition in the file, starting at 1.
        kind: What is defined.
        name: The name of the definition.
        decorators: The dotted names of the decorators of the definition, such as `app.command`,
            without their arguments. A decorator that is not a dotted name is left out.
        owner: The class that the definition is in, by its name in the module (`Outer.Inner` for a
            nested class), or None if it is not in a class.
        bases: For a class, the dotted names of its base classes as they are written, without their
            type arguments. A base class that is not a dotted name is left out.
        unpacked: For a variable or a class attribute, whether it is first assigned by its name in an
            unpacking, such as `second` in `first, second = pair()`, where Python needs a name to bind.
    """

    path: Path
    line: int
    kind: Literal["function", "class", "method", "variable", "attribute"]
    name: str
    decorators: tuple[str, ...] = ()
    owner: str | None = None
    bases: tuple[str, ...] = ()
    unpacked: bool = False

    @property
    def qualified_name(self) -> str:
        """
        The name of the definition in its module, such as `Outer.Inner` for a nested class.
        """
        return self.name if self.owner is None else f"{self.owner}.{self.name}"


def decorator_names(node: ast.ClassDef | ast.FunctionDef | ast.AsyncFunctionDef) -> tuple[str, ...]:
    """
    Get the dotted names of the decorators of a class or function.
    """
    names = (_dotted_name(decorator) for decorator in node.decorator_list)
    return tuple(name for name in names if name is not None)


def _dotted_name(expression: ast.expr) -> str | None:
    """
    Get the dotted name of a decorator, such as `app.command` for `@app.command("name")`.

    Returns:
        The dotted name, or None if the decorator is not a dotted name, for example `@decorators[0]`.
    """
    match expression:
        case ast.Call(func=function):
            return _dotted_name(function)
        case ast.Name(id=name):
            return name
        case ast.Attribute(value=value, attr=name):
            prefix = _dotted_name(value)
            return None if prefix is None else f"{prefix}.{name}"
        case _:
            return None


def base_name(expression: ast.expr) -> str | None:
    """
    Get the dotted name of a base class, such as `pydantic.BaseModel` for `pydantic.BaseModel[int]`.

    Returns:
        The dotted name, or None if the base class is not a dotted name, for example `make_base()`.
    """
    match expression:
        case ast.Subscript(value=value):
            return base_name(value)
        case ast.Name(id=name):
            return name
        case ast.Attribute(value=value, attr=name):
            prefix = base_name(value)
            return None if prefix is None else f"{prefix}.{name}"
        case _:
            return None
