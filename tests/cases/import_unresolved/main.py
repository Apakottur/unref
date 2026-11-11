# An import from a module that is not in the project does not refer to a definition in it.
from missing import function
from os import path

print(function(), path)
