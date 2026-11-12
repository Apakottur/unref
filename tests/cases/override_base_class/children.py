import models
import pkg.base
from models import Direct
from pkg import Base, Missing, TooFar

from cycle_first import Cycle

# More dots than there are packages: the import is not resolved.
from .... import Beyond


# A base class is followed through the imports of the project.
class FromImport(Direct):
    model_config = {}


class FromModule(models.Child):
    model_config = {}


class FromSubmodule(pkg.base.Base):
    model_config = {}


# `pkg` imports `Base` from `pkg.base`.
class FromReexport(Base):
    model_config = {}


# A name that the module does not define, and an import that leads nowhere.
class FromMissing(Missing):
    model_config = {}  # expect: unused attribute


class FromUndefined(Undefined):
    model_config = {}  # expect: unused attribute


class FromCycle(Cycle):
    model_config = {}  # expect: unused attribute


class FromBeyond(Beyond):
    model_config = {}  # expect: unused attribute


# `pkg` imports `TooFar` with more dots than there are packages.
class FromTooFar(TooFar):
    model_config = {}  # expect: unused attribute


print(FromImport, FromModule, FromSubmodule, FromReexport, FromMissing, FromUndefined, FromCycle, FromBeyond, FromTooFar)
