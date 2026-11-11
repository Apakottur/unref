import ast


def find_references(module: ast.Module) -> set[str]:
    """
    Find the names that a Python module refers to.

    A name is referred to when it is read, as a plain name (`name`) or as an attribute (`value.name`),
    or when it is imported from another module (`from module import name`).

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
            case _:
                pass
    return names
