# Two modules that import a name from each other, which is never defined.
from cycle_second import Cycle

print(Cycle)
