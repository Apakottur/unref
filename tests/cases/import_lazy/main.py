# A lazy import refers to the definition of the module that it imports from, as an import does.
lazy import other
lazy from helpers import Class as Renamed
lazy from helpers import function

__lazy_modules__ = ["json"]

print(function(), Renamed(), other.used())
