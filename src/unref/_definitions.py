import ast
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal

from unref._models import Finding


@dataclass
class _Scope:
    """
    The scope that a node is in.

    Attributes:
        kind: The kind of the closest enclosing scope.
        names: The variables of a module, or the attributes of a class, that were already found. A
            method shares the names of its class, so that an attribute is found once.
        instance: In a method, the name of its first parameter, usually `self` or `cls`.
    """

    kind: Literal["module", "class", "function"]
    names: set[str] = field(default_factory=set)
    instance: str | None = None


def find_definitions(path: Path, module: ast.Module) -> list[Finding]:
    """
    Find the functions, classes, methods, variables and attributes that a Python module defines.

    Local variables and imports are left out.

    Args:
        path: The file of the module.
        module: The parsed module.

    Returns:
        A finding for each definition, in the order of the file.
    """
    findings: list[Finding] = []
    _collect(module, path=path, scope=_Scope(kind="module"), findings=findings)
    return findings


def _collect(node: ast.AST, *, path: Path, scope: _Scope, findings: list[Finding]) -> None:
    """
    Add a finding for each definition under a node.

    Args:
        node: The node to search under.
        path: The file of the node.
        scope: The scope of the node.
        findings: The findings to add to.
    """
    for child in ast.iter_child_nodes(node):
        match child:
            case ast.ClassDef():
                findings.append(Finding(path=path, line=child.lineno, kind="unused class", name=child.name))
                _collect(child, path=path, scope=_Scope(kind="class"), findings=findings)
            case ast.FunctionDef() | ast.AsyncFunctionDef():
                if scope.kind == "class":
                    kind = "unused method"
                    parameters = [*child.args.posonlyargs, *child.args.args]
                    instance = parameters[0].arg if parameters else None
                    function_scope = _Scope(kind="function", names=scope.names, instance=instance)
                else:
                    kind = "unused function"
                    function_scope = _Scope(kind="function")
                findings.append(Finding(path=path, line=child.lineno, kind=kind, name=child.name))
                _collect(child, path=path, scope=function_scope, findings=findings)
            case ast.Assign():
                for target in child.targets:
                    _collect_target(target, path=path, scope=scope, findings=findings)
            case ast.AnnAssign():
                _collect_target(child.target, path=path, scope=scope, findings=findings)
            case _:
                _collect(child, path=path, scope=scope, findings=findings)


def _collect_target(target: ast.expr, *, path: Path, scope: _Scope, findings: list[Finding]) -> None:
    """
    Add a finding for each variable or attribute that an assignment target defines.

    Args:
        target: The assignment target.
        path: The file of the target.
        scope: The scope of the assignment.
        findings: The findings to add to.
    """
    match target:
        case ast.Name(id=name) if scope.kind != "function":
            kind = "unused variable" if scope.kind == "module" else "unused attribute"
        case ast.Attribute(value=ast.Name(id=owner), attr=name) if owner == scope.instance:
            kind = "unused attribute"
        case ast.Tuple(elts=elements) | ast.List(elts=elements):
            for element in elements:
                _collect_target(element, path=path, scope=scope, findings=findings)
            return
        case ast.Starred(value=value):
            _collect_target(value, path=path, scope=scope, findings=findings)
            return
        case _:
            # A local variable, or an assignment into an object that is defined elsewhere.
            return

    # A variable or attribute is found once, where it is first assigned.
    if name not in scope.names:
        scope.names.add(name)
        findings.append(Finding(path=path, line=target.lineno, kind=kind, name=name))
