def by_module():
    return 1


def by_renamed_module():
    return 2


def unused():  # expect: 5 [unused-function]
    return 3


def by_object():  # expect: 5 [unused-function]
    return 4


def hidden_by_parameter():  # expect: 5 [unused-function]
    return 5


def hidden_by_comprehension():  # expect: 5 [unused-function]
    return 6


def external():  # expect: 5 [unused-function]
    return 7


def too_far():  # expect: 5 [unused-function]
    return 8


class Holder:
    on_class = 1
    unused_on_class = 2  # expect: 5 [unused-attribute]

    def method(self):
        return self

    def external(self):
        return self

    def too_far(self):
        return self
