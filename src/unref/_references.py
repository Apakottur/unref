import ast


def find_references(module: ast.Module) -> set[str]:
    """
    Find the names that a Python module refers to.

    A name is referred to when it is read, as a plain name (`name`) or as an attribute (`value.name`),
    when it is imported from another module (`from module import name`), or when it is listed in
    `__all__`.

    A read of a local variable or a parameter is left out, since it does not refer to a definition.

    Args:
        module: The parsed module.

    Returns:
        The names.
    """
    names: set[str] = set()
    _visit(module, local_names=frozenset(), names=names)
    return names


def _visit(node: ast.AST, *, local_names: frozenset[str], names: set[str]) -> None:
    """
    Add the names that a node, and everything under it, refers to.

    Args:
        node: The node.
        local_names: The local variables and parameters that are visible at the node.
        names: The names to add to.
    """
    match node:
        case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.Lambda():
            _visit_function(node, local_names=local_names, names=names)
            return
        case ast.ListComp() | ast.SetComp() | ast.DictComp() | ast.GeneratorExp():
            # The variables of a comprehension are local to it.
            local_names |= _bound_names([node])
        case ast.Name(id=name, ctx=ast.Load()):
            if name not in local_names:
                names.add(name)
        case ast.Attribute(attr=name, ctx=ast.Load()):
            names.add(name)
        case ast.ImportFrom(names=aliases):
            names.update(alias.name for alias in aliases)
        case ast.Assign(targets=targets, value=value) if any(map(_is_all, targets)):
            names.update(_string_literals(value))
        case ast.AnnAssign(target=target, value=value) | ast.AugAssign(target=target, value=value) if _is_all(target):
            names.update(_string_literals(value))
        case _:
            pass

    for child in ast.iter_child_nodes(node):
        _visit(child, local_names=local_names, names=names)


def _visit_function(
    function: ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda,
    *,
    local_names: frozenset[str],
    names: set[str],
) -> None:
    """
    Add the names that a function or lambda refers to.

    Args:
        function: The function or lambda.
        local_names: The local variables and parameters that are visible where the function is defined.
        names: The names to add to.
    """
    arguments = function.args
    parameters = [
        *arguments.posonlyargs,
        *arguments.args,
        *arguments.kwonlyargs,
        *filter(None, [arguments.vararg, arguments.kwarg]),
    ]
    body: list[ast.AST] = [function.body] if isinstance(function, ast.Lambda) else [*function.body]

    # The decorators, the default values and the annotations are evaluated where the function is
    # defined, so they do not see its local variables.
    outside: list[ast.AST | None] = [
        *arguments.defaults,
        *arguments.kw_defaults,
        *(parameter.annotation for parameter in parameters),
    ]
    if not isinstance(function, ast.Lambda):
        outside += [*function.decorator_list, function.returns]
    for node in outside:
        if node is not None:
            _visit(node, local_names=local_names, names=names)

    local_names |= {parameter.arg for parameter in parameters} | _bound_names(body)
    for node in body:
        _visit(node, local_names=local_names, names=names)


def _bound_names(nodes: list[ast.AST]) -> frozenset[str]:
    """
    Get the names of the variables that the given nodes bind, without those of the functions and
    classes that are nested in them.

    A nested function or class is a definition and not a variable, so its name is left out, and a read
    of it is a reference. A name that is declared `global` is left out as well.
    """
    bound: set[str] = set()
    global_names: set[str] = set()

    stack = list(nodes)
    while stack:
        node = stack.pop()
        match node:
            case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef() | ast.Lambda():
                continue
            case ast.Name(id=name, ctx=ast.Store() | ast.Del()):
                bound.add(name)
            case ast.alias(name=name, asname=alias):
                # `import package.module` binds `package`.
                bound.add(alias or name.partition(".")[0])
            case (
                ast.ExceptHandler(name=str(name))
                | ast.MatchAs(name=str(name))
                | ast.MatchStar(name=str(name))
                | ast.MatchMapping(rest=str(name))
            ):
                bound.add(name)
            case ast.Global(names=declared):
                global_names.update(declared)
            case _:
                pass
        stack.extend(ast.iter_child_nodes(node))

    return frozenset(bound - global_names)


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
