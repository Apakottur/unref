import ast
from dataclasses import dataclass, field


@dataclass
class References:
    """
    What a Python module refers to.

    Attributes:
        names: The names that are read as a plain name (`name`), without the local variables and
            parameters, and the names that are listed in `__all__`. They refer to the module itself.
        attributes: The names that are read as an attribute (`value.name`).
        imports: The names that are imported from another module (`from module import name`), each
            with the module, as it is written, and its number of leading dots. A star import has the
            name `*`.
        has_all: Whether the module defines `__all__`.
    """

    names: set[str] = field(default_factory=set)
    attributes: set[str] = field(default_factory=set)
    imports: set[tuple[str | None, int, str]] = field(default_factory=set)
    has_all: bool = False


def find_references(module: ast.Module) -> References:
    """
    Find what a Python module refers to.

    Args:
        module: The parsed module.

    Returns:
        The references.
    """
    references = References()
    _visit(module, local_names=frozenset(), references=references)
    return references


def _visit(node: ast.AST, *, local_names: frozenset[str], references: References) -> None:
    """
    Add what a node, and everything under it, refers to.

    Args:
        node: The node.
        local_names: The local variables and parameters that are visible at the node.
        references: The references to add to.
    """
    match node:
        case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.Lambda():
            _visit_function(node, local_names=local_names, references=references)
            return
        case ast.ListComp() | ast.SetComp() | ast.DictComp() | ast.GeneratorExp():
            # The variables of a comprehension are local to it.
            local_names |= _bound_names([node])
        case ast.Name(id=name, ctx=ast.Load()):
            if name not in local_names:
                references.names.add(name)
        case ast.Attribute(attr=name, ctx=ast.Load()):
            references.attributes.add(name)
        case ast.ImportFrom(module=source, level=level, names=aliases):
            references.imports.update((source, level, alias.name) for alias in aliases)
        case ast.Assign(targets=targets, value=value) if any(map(_is_all, targets)):
            references.has_all = True
            references.names.update(_string_literals(value))
        case ast.AnnAssign(target=target, value=value) | ast.AugAssign(target=target, value=value) if _is_all(target):
            references.has_all = True
            references.names.update(_string_literals(value))
        case _:
            pass

    for child in ast.iter_child_nodes(node):
        _visit(child, local_names=local_names, references=references)


def _visit_function(
    function: ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda,
    *,
    local_names: frozenset[str],
    references: References,
) -> None:
    """
    Add what a function or lambda refers to.

    Args:
        function: The function or lambda.
        local_names: The local variables and parameters that are visible where the function is defined.
        references: The references to add to.
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
            _visit(node, local_names=local_names, references=references)

    local_names |= {parameter.arg for parameter in parameters} | _bound_names(body)
    for node in body:
        _visit(node, local_names=local_names, references=references)


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
