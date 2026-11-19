# A class that a function declares `global` is a class of the module, also when it is read there.
import logging


def define():
    global Shared

    class Shared:
        def helper(self):
            pass

        def other(self):  # expect: 13 [unused-method]
            pass


def other():
    class Shared(logging.Handler):
        def emit(self, record):
            pass

    return Shared()


class Child(Shared):
    def emit(self, record):  # expect: 9 [unused-method]
        pass


print(define(), other(), Shared().helper(), Child())
