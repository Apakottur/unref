from .... import TooFar
from .base import Base

# A star import with more dots than there are packages is not followed.
from .... import *

print(Base, TooFar)
