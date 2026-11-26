import logging
import socketserver
import sys
from io import StringIO
from typing import TYPE_CHECKING

# A variable of the module that is assigned a class outside the project, as a base class.
Handler = logging.StreamHandler


class FromAlias(Handler):
    def emit(self, record):
        return record

    def own_method(self):  # expect: 9 [unused-method]
        return self


# A variable that is assigned in both branches of an `if`, once with type arguments, is each value.
if TYPE_CHECKING:
    StreamHandler = logging.StreamHandler[StringIO]
else:
    StreamHandler = logging.StreamHandler


class FromBranches(StreamHandler):
    def emit(self, record):
        return record

    def handleError(self, record):
        return record


# A variable in one branch and a class of that name in the other are both base classes.
if sys.platform != "win32":
    ForkingMixIn = socketserver.ForkingMixIn
else:

    class ForkingMixIn:
        pass


class Server(ForkingMixIn):
    def __init__(self):
        # An attribute that the base class outside the project reads.
        self.max_children = 4
        self.own_attribute = None  # expect: 14 [unused-attribute]


# A variable that is assigned something other than a dotted name is not followed.
Called = logging.getLogger()


class FromCall(Called):
    def emit(self, record):  # expect: 9 [unused-method]
        return record


FromAlias, FromBranches, Server, FromCall
