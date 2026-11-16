# A chain of unused functions is shown from its top: deleting it leaves the next one unused, in turn.
def chain_top():  # expect: 5 [unused-function]
    return chain_middle()


def chain_middle():  # expect: 5 [unused-function] transitive
    return chain_bottom()


def chain_bottom():  # expect: 5 [unused-function] transitive
    return 1


# A definition that several unused functions reach is transitive, whichever is deleted first.
def first_caller():  # expect: 5 [unused-function]
    return shared() + SHARED_VARIABLE


def second_caller():  # expect: 5 [unused-function]
    return shared() + SharedClass().shared_method()


def shared():  # expect: 5 [unused-function] transitive
    return 2


SHARED_VARIABLE = 3  # expect: 1 [unused-variable] transitive


class SharedClass:  # expect: 7 [unused-class] transitive
    def shared_method(self):  # expect: 9 [unused-method] transitive
        return 4


# What is defined in an unused class or function goes with it.
class UnusedClass:  # expect: 7 [unused-class]
    attribute = 5  # expect: 5 [unused-attribute] transitive

    def method(self):  # expect: 9 [unused-method] transitive
        self.instance_attribute = 6  # expect: 14 [unused-attribute] transitive

    class Inner:  # expect: 11 [unused-class] transitive
        def inner_method(self):  # expect: 13 [unused-method] transitive
            return 7


def unused_outer():  # expect: 5 [unused-function]
    def nested():  # expect: 9 [unused-function] transitive
        return 8

    class NestedClass:  # expect: 11 [unused-class] transitive
        def nested_method(self):  # expect: 13 [unused-method] transitive
            return 9

    return 10


# Unused definitions that only reach each other are all shown, since none of them is reached from
# outside.
def loop_one():  # expect: 5 [unused-function]
    return loop_two()


def loop_two():  # expect: 5 [unused-function]
    return loop_one()


def recursive():  # expect: 5 [unused-function]
    return recursive()


# A cycle that an unused function outside of it reaches is transitive.
def into_cycle():  # expect: 5 [unused-function]
    return reached_one()


def reached_one():  # expect: 5 [unused-function] transitive
    return reached_two()


def reached_two():  # expect: 5 [unused-function] transitive
    return reached_one()


# A cycle can go through what is defined in a class, which is then shown as well.
class Ping:  # expect: 7 [unused-class]
    def make(self):  # expect: 9 [unused-method]
        return Pong()


class Pong:  # expect: 7 [unused-class]
    def make(self):  # expect: 9 [unused-method]
        return Ping()


# A definition that something used reaches is used, even if an unused function reaches it too.
def unused_and_used_caller():  # expect: 5 [unused-function]
    return also_used()


def also_used():
    return 11


# An ignored definition is used, so it does not hide what is in it, and what it calls is used.
class IgnoredClass:  # unref: ignore[unused-class]
    def method_of_ignored(self):  # expect: 9 [unused-method]
        return from_ignored()


def ignored_function():  # unref: ignore[unused-function]
    return from_ignored()


def from_ignored():
    return 12


# An ignore comment on a transitive finding ignores it, whether it is shown or not.
def calls_ignored():  # expect: 5 [unused-function]
    return ignored_transitive()


def ignored_transitive():  # unref: ignore[unused-function]
    return 13


class WithIgnoredMember:  # expect: 7 [unused-class]
    ignored_member = 14  # unref: ignore[unused-attribute]


print(also_used())
