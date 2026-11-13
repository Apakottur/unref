from collections.abc import Hashable, Iterator, Mapping, Sequence
from typing import TypeVar

Node = TypeVar("Node", bound=Hashable)


def transitive(successors: Mapping[Node, Sequence[Node]]) -> set[Node]:
    """
    Find the nodes of a directed graph that are reached from outside their strongly connected component.

    The nodes that only reach each other, such as two functions that call each other, form a component.
    A component that no other component reaches is a root, and the others are transitive: removing the
    roots, and then what they alone reached, in turn, removes them.

    Args:
        successors: The nodes that each node reaches. Every node is a key.

    Returns:
        The nodes of the components that another component reaches.
    """
    component = _components(successors)
    reached = {
        component[successor]
        for node, nodes in successors.items()
        for successor in nodes
        if component[successor] != component[node]
    }
    return {node for node in successors if component[node] in reached}


def _components(successors: Mapping[Node, Sequence[Node]]) -> dict[Node, Node]:
    """
    Find the strongly connected components of a directed graph, with Tarjan's algorithm.

    The walk keeps its own stack rather than recursing, so that a long chain does not reach the
    recursion limit.

    Args:
        successors: The nodes that each node reaches. Every node is a key.

    Returns:
        The component of each node, by one of its nodes.
    """
    index: dict[Node, int] = {}
    low: dict[Node, int] = {}
    stack: list[Node] = []
    on_stack: set[Node] = set()
    component: dict[Node, Node] = {}

    def visit(node: Node) -> tuple[Node, Iterator[Node]]:
        """Number a node and put it on the stack, and get the walk of its successors."""
        index[node] = low[node] = len(index)
        stack.append(node)
        on_stack.add(node)
        return node, iter(successors[node])

    for start in successors:
        if start in index:
            continue
        walk = [visit(start)]
        while walk:
            node, pending = walk[-1]
            for successor in pending:
                if successor not in index:
                    walk.append(visit(successor))
                    break
                if successor in on_stack:
                    low[node] = min(low[node], index[successor])
            else:
                walk.pop()
                if walk:
                    parent = walk[-1][0]
                    low[parent] = min(low[parent], low[node])
                if low[node] == index[node]:
                    while True:
                        member = stack.pop()
                        on_stack.discard(member)
                        component[member] = node
                        if member == node:
                            break
    return component
