import ast
from ast import NodeTransformer, NodeVisitor


def is_call(node):
    return isinstance(node, ast.Call)


# `ast.NodeVisitor.visit` calls the method named after the class of the node, `visit_` and the name.
class Calls(ast.NodeVisitor):
    def visit_Call(self, node):
        is_call(node)
        self.generic_visit(node)

    # Other methods are reported as usual.
    def report(self):  # expect: 9 [unused-method]
        pass


class Names(NodeVisitor):
    def visit_Name(self, node):
        node.id


# `NodeTransformer` is a `NodeVisitor`.
class Constants(NodeTransformer):
    def visit_Constant(self, node):
        return node


# A subclass of a visitor of the project.
class MoreCalls(Calls):
    def visit_Attribute(self, node):
        node.attr


# Not a visitor: a method named like one is reported.
class Plain:
    def visit_Call(self, node):  # expect: 9 [unused-method]
        node


tree = ast.parse("x")
for visitor in (Calls(), Names(), Constants(), MoreCalls(), Plain()):
    visitor.visit(tree)
