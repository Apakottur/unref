import logging

import lazy
import reexport
import vendored.outer
from vendored.outer import Outer as Imported


# A nested class is followed to the base classes that it inherits from.
class Outer:
    class Base(logging.Handler):
        pass


class Handler(Outer.Base):
    def emit(self, record):
        return record

    def own_method(self):  # expect: unused method
        return self


# A nested class of a module outside the project defines members, like any other class there.
class FromExternal(vendored.outer.Outer.Base):
    def emit(self, record):
        return record

    def custom(self):
        return self


class FromImported(Imported.Base):
    def custom(self):
        return self


# The class that a nested class is in is followed through a re-export, and through `__getattr__`.
class FromReexport(reexport.Outer.Base):
    def custom(self):
        return self


class FromLazy(lazy.Outer.Base):
    def custom(self):
        return self


# A name that the nested class does not define is not followed.
class FromMissing(Outer.Missing):
    def emit(self, record):  # expect: unused method
        return record


print(Handler, FromExternal, FromImported, FromReexport, FromLazy, FromMissing)
