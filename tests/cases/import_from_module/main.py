# An import refers to the definition of the module that it imports from.
from helpers import Class as Renamed
from helpers import function

print(function(), Renamed())
