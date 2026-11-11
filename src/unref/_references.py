import ast


def find_references(module: ast.Module) -> set[str]:
    """
    Find the names that a Python module refers to.

    A name is referred to when it is read, as a plain name (`name`) or as an attribute (`value.name`),
    when it is imported from another module (`from module import name`), or when it is listed in
    `__all__`.

    Args:
        module: The parsed module.

    Returns:
        The names.
    """
    names: set[str] = set()
    for node in ast.walk(module):
        match node:
            case ast.Name(id=name, ctx=ast.Load()) | ast.Attribute(attr=name, ctx=ast.Load()):
                names.add(name)
            case ast.ImportFrom(names=aliases):
                names.update(alias.name for alias in aliases)
            case ast.Assign(targets=targets, value=value) if any(map(_is_all, targets)):
                names.update(_string_literals(value))
            case ast.AnnAssign(target=target, value=value) | ast.AugAssign(target=target, value=value) if _is_all(
                target
            ):
                names.update(_string_literals(value))
            case _:
                pass
    return names


def _is_all(target: ast.expr) -> bool:
    """
    Check whether an assignment target is the `__all__` variable.
    """
    return isinstance(target, ast.Name) and target.id == "__all__"


def _string_literals(value: ast.expr | None) -> set[str]:
    """
    Get the string literals of a list or tuple literal.

    Anything else, such as a computed list, has none.
    """
    match value:
        case ast.List(elts=elements) | ast.Tuple(elts=elements):
            return {
                element.value
                for element in elements
                if isinstance(element, ast.Constant) and isinstance(element.value, str)
            }
        case _:
            return set()
