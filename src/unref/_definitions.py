import ast
from pathlib import Path

from unref._models import Finding


def find_definitions(path: Path) -> list[Finding]:
    """
    Find the functions, classes and methods that a Python file defines.

    Args:
        path: The Python file.

    Returns:
        A finding for each definition, in the order of the file.
    """
    findings: list[Finding] = []
    _collect(ast.parse(path.read_bytes(), filename=str(path)), path=path, in_class=False, findings=findings)
    return findings


def _collect(node: ast.AST, *, path: Path, in_class: bool, findings: list[Finding]) -> None:
    """
    Add a finding for each definition under a node.

    Args:
        node: The node to search under.
        path: The file of the node.
        in_class: Whether the closest enclosing definition is a class, which makes a function a method.
        findings: The findings to add to.
    """
    for child in ast.iter_child_nodes(node):
        match child:
            case ast.ClassDef():
                findings.append(Finding(path=path, line=child.lineno, kind="unused class", name=child.name))
                _collect(child, path=path, in_class=True, findings=findings)
            case ast.FunctionDef() | ast.AsyncFunctionDef():
                kind = "unused method" if in_class else "unused function"
                findings.append(Finding(path=path, line=child.lineno, kind=kind, name=child.name))
                _collect(child, path=path, in_class=False, findings=findings)
            case _:
                _collect(child, path=path, in_class=in_class, findings=findings)
