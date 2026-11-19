# A lazy import refers to the definition of the module that it imports from, as an import does.
lazy import import_lazy_other
lazy from import_lazy_helpers import Class as Renamed
lazy from import_lazy_helpers import function

__lazy_modules__ = ["json"]

function(), Renamed(), import_lazy_other.used()
