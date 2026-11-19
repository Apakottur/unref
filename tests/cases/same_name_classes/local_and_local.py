# Classes of one name in two functions are two classes, whichever comes first.
import logging


def external_first():
    class Handler(logging.Handler):
        def emit(self, record):
            pass

    return Handler()


def plain_second():
    class Handler:
        def emit(self, record):  # expect: 13 [unused-method]
            pass

    return Handler()


def plain_first():
    class Sink:
        def emit(self, record):  # expect: 13 [unused-method]
            pass

    return Sink()


def external_second():
    class Sink(logging.Handler):
        def emit(self, record):
            pass

    return Sink()


print(external_first(), plain_second(), plain_first(), external_second())
