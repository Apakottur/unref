import models
import pkg.base
from models import Direct, Outer
from pkg import Base, Missing, TooFar

from cycle_first import Cycle

# More dots than there are packages: the import is not resolved.
from .... import Beyond


# A base class is followed through the imports of the project.
class FromImport(Direct):
    table_name = {}


class FromModule(models.Child):
    table_name = {}


class FromSubmodule(pkg.base.Base):
    table_name = {}


class FromNested(Outer.Inner):
    table_name = {}


class FromNestedModule(models.Outer.Inner):
    table_name = {}


# `pkg` imports `Base` from `pkg.base`.
class FromReexport(Base):
    table_name = {}


# A name that the module does not define, and an import that leads nowhere.
class FromMissing(Missing):
    table_name = {}  # expect: unused attribute


class FromUndefined(Undefined):
    table_name = {}  # expect: unused attribute


class FromCycle(Cycle):
    table_name = {}  # expect: unused attribute


class FromBeyond(Beyond):
    table_name = {}  # expect: unused attribute


# `pkg` imports `TooFar` with more dots than there are packages.
class FromTooFar(TooFar):
    table_name = {}  # expect: unused attribute


print(FromImport, FromModule, FromNested, FromNestedModule, FromSubmodule, FromReexport, FromMissing, FromUndefined, FromCycle, FromBeyond, FromTooFar)
