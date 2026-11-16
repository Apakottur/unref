# One dot is the package of the module, and each further dot is one package up.
from . import IN_SUB
from .. import SHARED
from ..helpers import from_sub

# More dots than there are packages: the import is not resolved.
from .... import beyond

print(IN_SUB, SHARED, from_sub(), beyond)
