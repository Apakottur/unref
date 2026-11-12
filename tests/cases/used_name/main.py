class Model:  # expect: 7 [unused-class]
    # An exact name.
    model_config = {}
    model_configuration = {}  # expect: 5 [unused-attribute]
    config = {}  # expect: 5 [unused-attribute]


class Visitor:  # expect: 7 [unused-class]
    # A pattern, where `*` stands for any text.
    def visit_name(self):
        return 1

    def visit_(self):
        return 2

    def visit(self):  # expect: 9 [unused-method]
        return 3

    def revisit_name(self):  # expect: 9 [unused-method]
        return 4


def error_handler():
    return 5


def handler():  # expect: 5 [unused-function]
    return 6


# A pattern, where `?` stands for one character.
item_1 = 7
item_10 = 8  # expect: 1 [unused-variable]


# A name is matched with its case.
class TestCase:
    def __init__(self):
        self.visit_count = 9
        self.count = 10  # expect: 14 [unused-attribute]


class testcase:  # expect: 7 [unused-class]
    pass
