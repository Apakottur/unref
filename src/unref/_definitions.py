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
        line: The line of the name of the definition in the file, starting at 1.
        column: The column of the name of the definition in its line, in characters, starting at 1.
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
        annotation: For a variable or an attribute that is first assigned with an annotation, the
            dotted name of the annotation as it is written, without its type arguments, such as
            `ClassVar` for `ClassVar[int]`. An empty string if the annotation is not a dotted name, such
            as `int | None`, and None if there is no annotation.
        call: For a variable or an attribute that is first assigned the result of a call, the dotted
            name of what it calls as it is written, such as `mapped_column` for `mapped_column(Integer)`,
            or None if the value is not a call of a dotted name.
    """

    path: Path
    line: int
    column: int
    kind: Literal["function", "class", "method", "variable", "attribute"]
    name: str
    decorators: tuple[str, ...] = ()
    owner: str | None = None
    bases: tuple[str, ...] = ()
    unpacked: bool = False
    annotation: str | None = None
    call: str | None = None

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
    names = (dotted_name(decorator, unwrap=ast.Call) for decorator in node.decorator_list)
    return tuple(name for name in names if name is not None)


def annotation_name(annotation: ast.expr) -> str:
    """
    Get the dotted name of an annotation, without its type arguments, such as `ClassVar` for
    `ClassVar[int]`, also when the annotation is quoted. An empty string if it is not a dotted name.
    """
    if isinstance(annotation, ast.Constant) and isinstance(annotation.value, str):
        try:
            annotation = ast.parse(annotation.value.strip(), mode="eval").body
        except SyntaxError:
            return ""
    return dotted_name(annotation, unwrap=ast.Subscript) or ""


def dotted_name(expression: ast.expr, *, unwrap: type[ast.Call | ast.Subscript]) -> str | None:
    """
    Get the dotted name of a decorator or a base class.

    A decorator is unwrapped from a call, such as `app.command` for `@app.command("name")`, and a base
    class from a subscript, such as `pydantic.BaseModel` for `pydantic.BaseModel[int]`.

    Args:
        expression: The decorator or base class.
        unwrap: The kind of expression to unwrap: `ast.Call` for a decorator and `ast.Subscript` for a
            base class.

    Returns:
        The dotted name, or None if the expression is not a dotted name, for example `@decorators[0]`
        or `make_base()`.
    """
    match expression:
        case ast.Call(func=inner) | ast.Subscript(value=inner) if isinstance(expression, unwrap):
            return dotted_name(inner, unwrap=unwrap)
        case ast.Name(id=name):
            return name
        case ast.Attribute(value=value, attr=name):
            prefix = dotted_name(value, unwrap=unwrap)
            return None if prefix is None else f"{prefix}.{name}"
        case _:
            return None
