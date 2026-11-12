import pkg.sub.deeper.mod
import pkg.sub.mod
from pkg.sub.mod import by_absolute

from .sub.mod import Base


class Child(Base):
    pass


print(by_absolute(), pkg.sub.mod.by_attribute(), pkg.sub.deeper.mod.by_nested_attribute())
