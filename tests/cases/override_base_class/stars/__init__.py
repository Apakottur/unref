# A package whose modules star import it back, in a cycle.
from .first import *
from .second import *
