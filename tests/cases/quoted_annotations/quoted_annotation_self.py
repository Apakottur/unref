# A method can name its own class, or a class defined after it, in a quoted annotation. Like an
# annotation that is not quoted, the signature is read where the method is defined.
class Node:
    def __init__(self, parent: "Node | None") -> None:
        self.parent = parent

    def link(self, edge: "Edge") -> "Node":
        edge, self.parent
        return self


class Edge:
    pass


# An annotation in the body of a method is read only when the method runs.
class Tree:  # expect: 7 [unused-class]
    def root(self):  # expect: 9 [unused-method]
        tree: "Tree" = self
        return tree


Node(None).link
