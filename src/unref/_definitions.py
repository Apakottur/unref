import ast
from dataclasses import dataclass, field
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
    """

    path: Path
    line: int
    kind: Literal["function", "class", "method", "variable", "attribute"]
    name: str
    decorators: tuple[str, ...] = ()
    owner: str | None = None
    bases: tuple[str, ...] = ()

    @property
    def qualified_name(self) -> str:
        """
        The name of the definition in its module, such as `Outer.Inner` for a nested class.
        """
        return self.name if self.owner is None else f"{self.owner}.{self.name}"


@dataclass
class _Scope:
    """
    The scope that a node is in.

    Attributes:
        kind: The kind of the closest enclosing scope.
        names: The variables of a module, or the attributes of a class, that were already found. A
            method shares the names of its class, so that an attribute is found once.
        instance: In a method, the name of its first parameter, usually `self` or `cls`.
        owner: In a class or a method, the name of the class in the module.
    """

    kind: Literal["module", "class", "function"]
    names: set[str] = field(default_factory=set)
    instance: str | None = None
    owner: str | None = None


def find_definitions(path: Path, module: ast.Module) -> list[Definition]:
    """
    Find the functions, classes, methods, variables and attributes that a Python module defines.

    Local variables and imports are left out.

    Args:
        path: The file of the module.
        module: The parsed module.

    Returns:
        The definitions, in the order of the file.
    """
    definitions: list[Definition] = []
    _collect(module, path=path, scope=_Scope(kind="module"), definitions=definitions)
    return definitions


def _collect(node: ast.AST, *, path: Path, scope: _Scope, definitions: list[Definition]) -> None:
    """
    Add the definitions under a node.

    Args:
        node: The node to search under.
        path: The file of the node.
        scope: The scope of the node.
        definitions: The definitions to add to.
    """
    for child in ast.iter_child_nodes(node):
        match child:
            case ast.ClassDef():
                definition = Definition(
                    path=path,
                    line=child.lineno,
                    kind="class",
                    name=child.name,
                    decorators=_decorators(child),
                    owner=scope.owner if scope.kind == "class" else None,
                    bases=tuple(filter(None, map(_base_name, child.bases))),
                )
                definitions.append(definition)
                class_scope = _Scope(kind="class", owner=definition.qualified_name)
                _collect(child, path=path, scope=class_scope, definitions=definitions)
            case ast.FunctionDef() | ast.AsyncFunctionDef():
                kind: Literal["function", "method"]
                if scope.kind == "class":
                    kind = "method"
                    parameters = [*child.args.posonlyargs, *child.args.args]
                    instance = parameters[0].arg if parameters else None
                    function_scope = _Scope(kind="function", names=scope.names, instance=instance, owner=scope.owner)
                else:
                    kind = "function"
                    function_scope = _Scope(kind="function")
                definitions.append(
                    Definition(
                        path=path,
                        line=child.lineno,
                        kind=kind,
                        name=child.name,
                        decorators=_decorators(child),
                        owner=scope.owner if scope.kind == "class" else None,
                    )
                )
                _collect(child, path=path, scope=function_scope, definitions=definitions)
            case ast.Assign():
                for target in child.targets:
                    _collect_target(target, path=path, scope=scope, definitions=definitions)
            case ast.AnnAssign():
                _collect_target(child.target, path=path, scope=scope, definitions=definitions)
            case _:
                _collect(child, path=path, scope=scope, definitions=definitions)


def _collect_target(target: ast.expr, *, path: Path, scope: _Scope, definitions: list[Definition]) -> None:
    """
    Add the variables and attributes that an assignment target defines.

    Args:
        target: The assignment target.
        path: The file of the target.
        scope: The scope of the assignment.
        definitions: The definitions to add to.
    """
    kind: Literal["variable", "attribute"]
    match target:
        case ast.Name(id=name) if scope.kind != "function":
            kind = "variable" if scope.kind == "module" else "attribute"
        case ast.Attribute(value=ast.Name(id=owner), attr=name) if owner == scope.instance:
            kind = "attribute"
        case ast.Tuple(elts=elements) | ast.List(elts=elements):
            for element in elements:
                _collect_target(element, path=path, scope=scope, definitions=definitions)
            return
        case ast.Starred(value=value):
            _collect_target(value, path=path, scope=scope, definitions=definitions)
            return
        case _:
            # A local variable, or an assignment into an object that is defined elsewhere.
            return

    # A variable or attribute is found once, where it is first assigned.
    if name not in scope.names:
        scope.names.add(name)
        definitions.append(Definition(path=path, line=target.lineno, kind=kind, name=name, owner=scope.owner))


def _decorators(node: ast.ClassDef | ast.FunctionDef | ast.AsyncFunctionDef) -> tuple[str, ...]:
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


def _base_name(expression: ast.expr) -> str | None:
    """
    Get the dotted name of a base class, such as `pydantic.BaseModel` for `pydantic.BaseModel[int]`.

    Returns:
        The dotted name, or None if the base class is not a dotted name, for example `make_base()`.
    """
    match expression:
        case ast.Subscript(value=value):
            return _base_name(value)
        case ast.Name(id=name):
            return name
        case ast.Attribute(value=value, attr=name):
            prefix = _base_name(value)
            return None if prefix is None else f"{prefix}.{name}"
        case _:
            return None
