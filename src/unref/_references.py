import ast
from dataclasses import dataclass, field

# A name that a module imports: the module it is imported from, as it is written, the number of leading
# dots, and the imported name. The name is None for `import module`, which binds the module itself.
Binding = tuple[str | None, int, str | None]

# The code that a reference is made from: the line of the `def` of the function that the reference is
# in, or None for code that runs when the module is imported.
Owner = int | None


@dataclass
class References:
    """
    What a Python module refers to.

    Each reference is made from an owner: the function that it is in, or the module itself.

    Attributes:
        names: The names that are read as a plain name (`name`), without the local variables and
            parameters, and the names that are listed in `__all__`. They refer to the module itself.
        attributes: The names that are read as an attribute (`value.name`), without those that are
            read from an imported name.
        imported_attributes: The attributes that are read from an imported name
            (`module.name.attribute`), each with what the imported name stands for and the names of
            the attributes, in order.
        imports: The names that are imported from another module (`from module import name`), each
            with the module, as it is written, and its number of leading dots. A star import has the
            name `*`.
        has_all: Whether the module defines `__all__`.
        bindings: The names that the module imports at its top level, each with what it stands for.
    """

    names: set[tuple[Owner, str]] = field(default_factory=set)
    attributes: set[tuple[Owner, str]] = field(default_factory=set)
    imported_attributes: set[tuple[Owner, Binding, tuple[str, ...]]] = field(default_factory=set)
    imports: set[tuple[Owner, str | None, int, str]] = field(default_factory=set)
    has_all: bool = False
    bindings: dict[str, Binding] = field(default_factory=dict)


def find_references(module: ast.Module) -> References:
    """
    Find what a Python module refers to.

    Args:
        module: The parsed module.

    Returns:
        The references.
    """
    references = References(bindings=_import_bindings([module]))
    _visit(module, owner=None, local_names=frozenset(), imports=references.bindings, references=references)
    return references


def _import_bindings(nodes: list[ast.AST]) -> dict[str, Binding]:
    """
    Get the names that the given nodes import, without those of the functions and classes that are
    nested in them.
    """
    bindings: dict[str, Binding] = {}
    stack = list(nodes)
    while stack:
        node = stack.pop()
        match node:
            case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef() | ast.Lambda():
                continue
            case ast.Import(names=aliases):
                for alias in aliases:
                    if alias.asname is None:
                        # `import package.module` binds `package`.
                        package = alias.name.partition(".")[0]
                        bindings[package] = (package, 0, None)
                    else:
                        bindings[alias.asname] = (alias.name, 0, None)
            case ast.ImportFrom(module=source, level=level, names=aliases):
                for alias in aliases:
                    bindings[alias.asname or alias.name] = (source, level, alias.name)
            case _:
                pass
        stack.extend(ast.iter_child_nodes(node))
    return bindings


def _visit(
    node: ast.AST,
    *,
    owner: Owner,
    local_names: frozenset[str],
    imports: dict[str, Binding],
    references: References,
) -> None:
    """
    Add what a node, and everything under it, refers to.

    Args:
        node: The node.
        owner: The function that the node is in, or None for code that runs when the module is imported.
        local_names: The local variables and parameters that are visible at the node.
        imports: The imported names that are visible at the node.
        references: The references to add to.
    """
    match node:
        case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.Lambda():
            _visit_function(node, owner=owner, local_names=local_names, imports=imports, references=references)
            return
        case ast.ListComp() | ast.SetComp() | ast.DictComp() | ast.GeneratorExp():
            # The variables of a comprehension are local to it.
            bound = _bound_names([node])
            local_names |= bound
            imports = {name: binding for name, binding in imports.items() if name not in bound}
        case ast.Name(id=name, ctx=ast.Load()):
            if name not in local_names:
                references.names.add((owner, name))
        case ast.Attribute(attr=name, ctx=ast.Load()):
            root, attributes = _attribute_chain(node)
            if root in imports:
                # Attributes of an imported name, which are resolved once every module is known.
                references.imported_attributes.add((owner, imports[root], attributes))
                return
            references.attributes.add((owner, name))
        case ast.ImportFrom(module=source, level=level, names=aliases):
            references.imports.update((owner, source, level, alias.name) for alias in aliases)
        case ast.Assign(targets=targets, value=value) if any(map(_is_all, targets)):
            references.has_all = True
            references.names.update((owner, name) for name in _string_literals(value))
        case ast.AnnAssign(target=target, value=value) | ast.AugAssign(target=target, value=value) if _is_all(target):
            references.has_all = True
            references.names.update((owner, name) for name in _string_literals(value))
        case _:
            pass

    for child in ast.iter_child_nodes(node):
        _visit(child, owner=owner, local_names=local_names, imports=imports, references=references)


def _visit_function(
    function: ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda,
    *,
    owner: Owner,
    local_names: frozenset[str],
    imports: dict[str, Binding],
    references: References,
) -> None:
    """
    Add what a function or lambda refers to.

    Args:
        function: The function or lambda.
        owner: The function that the function is defined in, or None.
        local_names: The local variables and parameters that are visible where the function is defined.
        imports: The imported names that are visible where the function is defined.
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
            _visit(node, owner=owner, local_names=local_names, imports=imports, references=references)

    # A local variable hides an imported name, and the function sees its own imports.
    bound = {parameter.arg for parameter in parameters} | _bound_names(body)
    local_names |= bound
    imports = {name: binding for name, binding in imports.items() if name not in bound} | _import_bindings(body)
    # The body runs only when the function is called. A lambda has no name, and is part of the code that
    # it is in.
    if not isinstance(function, ast.Lambda):
        owner = function.lineno
    for node in body:
        _visit(node, owner=owner, local_names=local_names, imports=imports, references=references)


def _attribute_chain(attribute: ast.Attribute) -> tuple[str | None, tuple[str, ...]]:
    """
    Split a chain of attributes, such as `package.module.name`, into the name that it starts with and
    its attributes.

    Returns:
        The name, which is None if the chain starts with something else, such as a call, and the
        attributes in order.
    """
    attributes: list[str] = []
    node: ast.expr = attribute
    while isinstance(node, ast.Attribute):
        attributes.insert(0, node.attr)
        node = node.value
    return (node.id if isinstance(node, ast.Name) else None), tuple(attributes)


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
