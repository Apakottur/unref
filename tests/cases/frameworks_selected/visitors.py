import ast


# The rules of the standard library are turned off.
class Calls(ast.NodeVisitor):
    def visit_Call(self, node):  # expect: 9 [unused-method]
        return node


Calls
