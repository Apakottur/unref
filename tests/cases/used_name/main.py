class Model:  # expect: unused class
    # An exact name.
    model_config = {}
    model_configuration = {}  # expect: unused attribute
    config = {}  # expect: unused attribute


class Visitor:  # expect: unused class
    # A pattern, where `*` stands for any text.
    def visit_name(self):
        return 1

    def visit_(self):
        return 2

    def visit(self):  # expect: unused method
        return 3

    def revisit_name(self):  # expect: unused method
        return 4


def error_handler():
    return 5


def handler():  # expect: unused function
    return 6


# A pattern, where `?` stands for one character.
item_1 = 7
item_10 = 8  # expect: unused variable


# A name is matched with its case.
class TestCase:
    def __init__(self):
        self.visit_count = 9
        self.count = 10  # expect: unused attribute


class testcase:  # expect: unused class
    pass
