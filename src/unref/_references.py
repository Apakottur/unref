import ast
from collections.abc import Callable
from dataclasses import dataclass, field
from functools import cache
from pathlib import Path
from typing import Any, Literal

from unref._definitions import Definition, base_name, decorator_names

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


def walk_module(path: Path, module: ast.Module) -> tuple[list[Definition], References]:
    """
    Find what a Python module defines and refers to.

    The module is walked once. A read of a name is kept with the scope that it is in until the scope
    ends, because a name that is bound anywhere in a function is local to all of it. It is then dropped
    if the name is local, and otherwise passed to the enclosing scope, until it reaches the module.

    The definitions are the functions, classes, methods, variables and attributes of the module. Local
    variables and imports are left out.

    Args:
        path: The file of the module.
        module: The parsed module.

    Returns:
        The definitions, in the order of the file, and the references.
    """
    references = References()
    walker = _Walker(path, references)
    walker.visit(module)

    # What reaches the module is read from the module itself, or from what it imports.
    scope = walker.scope
    references.bindings = scope.imports
    references.names |= scope.names
    for owner, root, attributes in scope.chains:
        if root in scope.imports:
            references.imported_attributes.add((owner, scope.imports[root], attributes))
        else:
            references.attributes.update((owner, attribute) for attribute in attributes)
            references.names.add((owner, root))
    return walker.definitions, references


@dataclass
class _Scope:
    """
    The module, a class body, a function, a lambda or a comprehension, while it is walked.

    Attributes:
        kind: Whether the definitions in the scope are module variables, class attributes, or local
            variables that are left out. A lambda and a comprehension hold no definitions.
        bound: The names that the scope binds, which are its local variables.
        global_names: The names that the scope declares `global`.
        imports: The names that the scope imports, each with what it stands for.
        names: The names that are read in the scope, or in a scope nested in it, and that are not local
            to the scopes in between, each with its owner.
        chains: The chains of attributes, such as `package.module.name`, that start with a name and
            that are read in the scope, or in a scope nested in it, as for `names`. Each has its owner,
            the name and the attributes in order.
        enclosing: The scope that also binds the names that the scope binds. A comprehension binds its
            variables in the function that it is in as well.
        defined: The variables of a module, or the attributes of a class, that were already found. A
            method shares the names of its class, so that an attribute is found once.
        instance: In a method, the name of its first parameter, usually `self` or `cls`.
        class_name: In a class or a method, the name of the class in the module.
    """

    kind: Literal["module", "class", "function"] = "function"
    bound: set[str] = field(default_factory=set)
    global_names: set[str] = field(default_factory=set)
    imports: dict[str, Binding] = field(default_factory=dict)
    names: set[tuple[Owner, str]] = field(default_factory=set)
    chains: set[tuple[Owner, str, tuple[str, ...]]] = field(default_factory=set)
    enclosing: "_Scope | None" = None
    defined: set[str] = field(default_factory=set)
    instance: str | None = None
    class_name: str | None = None


class _Walker:
    """
    Walk a module once, and add what it defines and refers to.

    Attributes:
        path: The file of the module.
        definitions: The definitions to add to.
        references: The references to add to.
        scope: The scope that the node being walked is in.
        binder: The scope that the names bound at the node belong to, which holds its definitions. In a
            class body, and in what a function evaluates where it is defined, it is a scope of its own
            that is then dropped, since the names that they bind are not tracked.
        owner: The function that the node is in, or None for code that runs when the module is imported.
    """

    def __init__(self, path: Path, references: References) -> None:
        self.path = path
        self.definitions: list[Definition] = []
        self.references = references
        self.scope = _Scope(kind="module")
        self.binder = self.scope
        self.owner: Owner = None

    def visit(self, node: ast.AST) -> None:
        """
        Add what a node, and everything under it, refers to.
        """
        handler = _HANDLERS.get(type(node))
        if handler is None:
            self.visit_children(node)
        else:
            handler(self, node)

    def visit_children(self, node: ast.AST) -> None:
        """
        Add what the children of a node, and everything under them, refer to.
        """
        for name in _child_fields(type(node)):
            value = getattr(node, name, None)
            if isinstance(value, list):
                for item in value:
                    if isinstance(item, ast.AST):
                        self.visit(item)
            elif isinstance(value, ast.AST):
                self.visit(value)

    def visit_name(self, node: ast.Name) -> None:
        """
        Add a read of a name, or bind it.
        """
        if isinstance(node.ctx, ast.Load):
            self.scope.names.add((self.owner, node.id))
        else:
            self.bind(node.id)

    def visit_attribute(self, node: ast.Attribute) -> None:
        """
        Add a read of an attribute, with the attributes that it is read from.
        """
        if not isinstance(node.ctx, ast.Load):
            self.visit(node.value)
            return
        base, attributes = _attribute_chain(node)
        if isinstance(base, ast.Name):
            # Whether the chain starts with an imported name is known when the scope ends.
            self.scope.chains.add((self.owner, base.id, attributes))
        else:
            self.references.attributes.update((self.owner, attribute) for attribute in attributes)
            self.visit(base)

    def visit_function(self, function: ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda) -> None:
        """
        Add what a function or lambda refers to.
        """
        arguments = function.args
        parameters = [
            *arguments.posonlyargs,
            *arguments.args,
            *arguments.kwonlyargs,
            *filter(None, [arguments.vararg, arguments.kwarg]),
        ]

        # The decorators, the default values and the annotations are evaluated where the function is
        # defined, so they do not see its local variables, and what they bind is not tracked.
        outside: list[ast.AST | None] = [
            *arguments.defaults,
            *arguments.kw_defaults,
            *(parameter.annotation for parameter in parameters),
        ]
        body_scope = _Scope()
        if not isinstance(function, ast.Lambda):
            outside += [*function.decorator_list, function.returns]
            body_scope = self.define_function(function)
        binder, self.binder = self.binder, _Scope()
        for node in outside:
            if node is not None:
                self.visit(node)

        # The body runs only when the function is called. A lambda has no name, and is part of the code
        # that it is in.
        scope, owner = self.scope, self.owner
        self.scope = self.binder = body_scope
        if isinstance(function, ast.Lambda):
            self.visit(function.body)
        else:
            self.owner = function.lineno
            for node in function.body:
                self.visit(node)
        inner = self.scope
        self.scope, self.binder, self.owner = scope, binder, owner
        self.close(inner, {parameter.arg for parameter in parameters} | (inner.bound - inner.global_names))

    def visit_class(self, node: ast.ClassDef) -> None:
        """
        Add a class, and what it defines and refers to.

        The body of a class is not a scope whose reads are tracked: it sees the names of the code that it
        is in, and what it binds is an attribute of the class.
        """
        definition = Definition(
            path=self.path,
            line=node.lineno,
            kind="class",
            name=node.name,
            decorators=decorator_names(node),
            owner=self.binder.class_name if self.binder.kind == "class" else None,
            bases=tuple(filter(None, map(base_name, node.bases))),
        )
        self.definitions.append(definition)
        binder, self.binder = self.binder, _Scope(kind="class", class_name=definition.qualified_name)
        self.visit_children(node)
        self.binder = binder

    def visit_comprehension(self, node: ast.ListComp | ast.SetComp | ast.DictComp | ast.GeneratorExp) -> None:
        """
        Add what a comprehension refers to. Its variables are local to it.
        """
        scope, binder = self.scope, self.binder
        self.scope = self.binder = _Scope(enclosing=binder)
        self.visit_children(node)
        inner = self.scope
        self.scope, self.binder = scope, binder
        self.close(inner, inner.bound)

    def visit_import(self, node: ast.Import) -> None:
        """
        Bind the names that an `import` statement imports.
        """
        bindings: dict[str, Binding] = {}
        for alias in node.names:
            if alias.asname is None:
                # `import package.module` binds `package`.
                package = alias.name.partition(".")[0]
                bindings[package] = (package, 0, None)
            else:
                bindings[alias.asname] = (alias.name, 0, None)
        self.bind_imports(bindings)

    def visit_import_from(self, node: ast.ImportFrom) -> None:
        """
        Add the names that a `from` import imports, and bind them.
        """
        self.references.imports.update((self.owner, node.module, node.level, alias.name) for alias in node.names)
        self.bind_imports({alias.asname or alias.name: (node.module, node.level, alias.name) for alias in node.names})

    def visit_global(self, node: ast.Global) -> None:
        """
        Note the names that a `global` statement declares, which are not local variables.
        """
        self.binder.global_names.update(node.names)

    def visit_assign(self, node: ast.Assign | ast.AnnAssign | ast.AugAssign) -> None:
        """
        Add the variables and attributes that an assignment defines, the names that an assignment to
        `__all__` lists, and what the assignment refers to.
        """
        targets = node.targets if isinstance(node, ast.Assign) else [node.target]
        if not isinstance(node, ast.AugAssign):
            for target in targets:
                self.define_target(target)
        if any(map(_is_all, targets)):
            self.references.has_all = True
            self.references.names.update((self.owner, name) for name in _string_literals(node.value))
        self.visit_children(node)

    def visit_pattern_or_handler(
        self, node: ast.ExceptHandler | ast.MatchAs | ast.MatchStar | ast.MatchMapping
    ) -> None:
        """
        Bind the name that an `except` clause or a pattern captures, and add what it refers to.
        """
        name = node.rest if isinstance(node, ast.MatchMapping) else node.name
        if name is not None:
            self.bind(name)
        self.visit_children(node)

    def visit_match_class(self, node: ast.MatchClass) -> None:
        """
        Add a read of each attribute that a class pattern matches by keyword (`Class(name=pattern)`).
        """
        self.references.attributes.update((self.owner, name) for name in node.kwd_attrs)
        self.visit_children(node)

    def define_function(self, function: ast.FunctionDef | ast.AsyncFunctionDef) -> _Scope:
        """
        Add a function or a method.

        Returns:
            The scope of its body.
        """
        enclosing = self.binder
        kind: Literal["function", "method"] = "function"
        scope = _Scope()
        if enclosing.kind == "class":
            kind = "method"
            parameters = [*function.args.posonlyargs, *function.args.args]
            instance = parameters[0].arg if parameters else None
            scope = _Scope(defined=enclosing.defined, instance=instance, class_name=enclosing.class_name)
        self.definitions.append(
            Definition(
                path=self.path,
                line=function.lineno,
                kind=kind,
                name=function.name,
                decorators=decorator_names(function),
                owner=enclosing.class_name if enclosing.kind == "class" else None,
            )
        )
        return scope

    def define_target(self, target: ast.expr, *, in_unpacking: bool = False) -> None:
        """
        Add the variables and attributes that an assignment target defines.

        Args:
            target: The assignment target.
            in_unpacking: Whether the target is part of an unpacking.
        """
        scope = self.binder
        kind: Literal["variable", "attribute"]
        unpacked = False
        match target:
            case ast.Name(id=name) if scope.kind != "function":
                kind = "variable" if scope.kind == "module" else "attribute"
                unpacked = in_unpacking
            case ast.Attribute(value=ast.Name(id=owner), attr=name) if owner == scope.instance:
                kind = "attribute"
            case ast.Tuple(elts=elements) | ast.List(elts=elements):
                for element in elements:
                    self.define_target(element, in_unpacking=True)
                return
            case ast.Starred(value=value):
                self.define_target(value, in_unpacking=True)
                return
            case _:
                # A local variable, or an assignment into an object that is defined elsewhere.
                return

        # A variable or attribute is found once, where it is first assigned.
        if name not in scope.defined:
            scope.defined.add(name)
            self.definitions.append(
                Definition(
                    path=self.path,
                    line=target.lineno,
                    kind=kind,
                    name=name,
                    owner=scope.class_name,
                    unpacked=unpacked,
                )
            )

    def bind(self, name: str) -> None:
        """
        Bind a name in the current scope, and in the scopes that it binds names in as well.
        """
        scope: _Scope | None = self.binder
        while scope is not None:
            scope.bound.add(name)
            scope = scope.enclosing

    def bind_imports(self, bindings: dict[str, Binding]) -> None:
        """
        Bind the names that a statement imports.

        When a name is imported more than once, as by an `import` in a `try` and a fallback in its
        `except`, the first import is the one that counts.
        """
        for name, binding in bindings.items():
            self.bind(name)
            self.binder.imports.setdefault(name, binding)

    def close(self, inner: _Scope, local_names: set[str]) -> None:
        """
        End a scope that is nested in the current one: drop the reads of its local variables, resolve
        the chains of attributes that start with a name that it imports, and pass the rest to the
        current scope.

        Args:
            inner: The scope that ends.
            local_names: The local variables and parameters of the scope.
        """
        outer = self.scope
        outer.names.update(read for read in inner.names if read[1] not in local_names)
        for owner, root, attributes in inner.chains:
            if root in inner.imports:
                # Attributes of an imported name, which are resolved once every module is known.
                self.references.imported_attributes.add((owner, inner.imports[root], attributes))
            elif root in local_names:
                self.references.attributes.update((owner, attribute) for attribute in attributes)
            else:
                outer.chains.add((owner, root, attributes))


# The nodes that are walked other than by visiting their children.
_HANDLERS: dict[type[ast.AST], Callable[[_Walker, Any], None]] = {
    ast.Name: _Walker.visit_name,
    ast.Attribute: _Walker.visit_attribute,
    ast.FunctionDef: _Walker.visit_function,
    ast.AsyncFunctionDef: _Walker.visit_function,
    ast.Lambda: _Walker.visit_function,
    ast.ClassDef: _Walker.visit_class,
    ast.ListComp: _Walker.visit_comprehension,
    ast.SetComp: _Walker.visit_comprehension,
    ast.DictComp: _Walker.visit_comprehension,
    ast.GeneratorExp: _Walker.visit_comprehension,
    ast.Import: _Walker.visit_import,
    ast.ImportFrom: _Walker.visit_import_from,
    ast.Global: _Walker.visit_global,
    ast.Assign: _Walker.visit_assign,
    ast.AnnAssign: _Walker.visit_assign,
    ast.AugAssign: _Walker.visit_assign,
    ast.ExceptHandler: _Walker.visit_pattern_or_handler,
    ast.MatchAs: _Walker.visit_pattern_or_handler,
    ast.MatchStar: _Walker.visit_pattern_or_handler,
    ast.MatchMapping: _Walker.visit_pattern_or_handler,
    ast.MatchClass: _Walker.visit_match_class,
}


@cache
def _child_fields(node_type: type[ast.AST]) -> tuple[str, ...]:
    """
    Get the fields of a type of node that can hold other nodes.

    The context of a name or an attribute, such as `ast.Load`, is left out, and a constant has none,
    since they hold nothing.
    """
    if node_type is ast.Constant:
        return ()
    return tuple(name for name in node_type._fields if name != "ctx")


def _attribute_chain(attribute: ast.Attribute) -> tuple[ast.expr, tuple[str, ...]]:
    """
    Split a chain of attributes, such as `package.module.name`, into what it starts with and its
    attributes.

    Returns:
        What the chain starts with, such as a name or a call, and the attributes in order.
    """
    attributes: list[str] = []
    node: ast.expr = attribute
    while isinstance(node, ast.Attribute):
        attributes.append(node.attr)
        node = node.value
    return node, tuple(reversed(attributes))


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
