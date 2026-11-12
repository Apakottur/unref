import helpers


# What an unused function uses is unused as well, unless something else uses it.
def unused():  # expect: unused function
    return only_from_unused() + also_from_used() + helpers.only_from_unused() + Holder().only_from_unused()


def only_from_unused():  # expect: unused function
    return chain()


def chain():  # expect: unused function
    return 1


def also_from_used():
    return 2


class Holder:
    def only_from_unused(self):  # expect: unused method
        return self

    def from_used_method(self):
        return self

    def used(self):
        return self.from_used_method()

    def unused(self):  # expect: unused method
        return self.only_from_unused()


# A nested function is used only if the function that it is in is used and it is called.
def outer():
    def called():
        return 3

    def not_called():  # expect: unused function
        return from_not_called()

    return called()


def from_not_called():  # expect: unused function
    return 4


# The decorators, default values and annotations of an unused function run when it is defined, so what
# they use is used.
def in_default():
    return 5


def in_decorator(function):
    return function


@in_decorator
def decorated(value=in_default()):  # expect: unused function
    return value


# A lambda is part of the code that it is in.
from_lambda = lambda: in_lambda()  # noqa: E731


def in_lambda():
    return 6


# Code that runs when the module is imported uses what it refers to, in a class body as well.
class Body:  # expect: unused class
    value = from_class_body()  # expect: unused attribute


def from_class_body():
    return 7


print(also_from_used(), Holder().used(), outer(), from_lambda)
