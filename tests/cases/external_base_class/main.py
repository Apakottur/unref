import json
import logging
import logging.handlers
from logging import Handler as RenamedHandler

import not_installed
import reexport


# A member that a base class outside the project defines is used by the code of the base class.
class LogHandler(logging.Handler):
    def emit(self, record):
        return format_record(record)

    def __init__(self):
        super().__init__()
        # An attribute that the base class sets on its instances.
        self.formatter = None
        self.own_attribute = None  # expect: 14 [unused-attribute]

    def own_method(self):  # expect: 9 [unused-method]
        return self


# What only an override uses is used.
def format_record(record):
    return record


# The base class is found through a re-export: `json` imports `JSONEncoder` from `json.encoder`.
class Encoder(json.JSONEncoder):
    # An attribute that the base class defines in its body.
    item_separator = ";"

    def default(self, o):
        return o


# The base classes of a base class outside the project are followed:
# `RotatingFileHandler` inherits from `Handler` through other classes.
class Rotating(logging.handlers.RotatingFileHandler):
    def emit(self, record):
        return record

    def shouldRollover(self, record):
        return record


class Renamed(RenamedHandler):
    def emit(self, record):
        return record


# A class of the project inherits from the base classes of its base classes.
class Child(LogHandler):
    def emit(self, record):
        return record

    def handle_error(self, record):  # expect: 9 [unused-method]
        return record


# A base class that is not found leaves its subclass as it is.
class FromMissing(not_installed.Handler):
    def emit(self, record):  # expect: 9 [unused-method]
        return record


# Not a subclass of the base class.
class Plain:
    def emit(self, record):  # expect: 9 [unused-method]
        return record


def emit(record):  # expect: 5 [unused-function]
    return record


print(LogHandler, Encoder, Rotating, Renamed, Child, FromMissing, Plain)


# `json.encoder` is a module, not a package, so it has no `JSONEncoder` submodule.
class FromAttribute(json.encoder.JSONEncoder.Missing):
    def default(self, o):  # expect: 9 [unused-method]
        return o


print(FromAttribute)


# A base class is found through a star import.
class FromStar(reexport.Handler):
    def emit(self, record):
        return record


print(FromStar)
