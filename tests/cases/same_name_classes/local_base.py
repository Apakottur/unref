# A base class is looked up from the function of the class outwards, and then in the module.
import logging


class Base:
    def emit(self, record):  # expect: 9 [unused-method]
        pass


def make():
    class Base(logging.Handler):
        pass

    class Child(Base):
        def emit(self, record):
            pass

    return Child()


def outer():
    class Parent(logging.Handler):
        pass

    def inner():
        class Child(Parent):
            def emit(self, record):
                pass

        return Child()

    return inner()


def top_level():
    class Child(Base):
        def emit(self, record):  # expect: 13 [unused-method]
            pass

    return Child()


print(Base(), make(), outer(), top_level())
