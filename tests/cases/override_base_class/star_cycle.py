from stars.first import Nowhere, Starred


# `stars.first` star imports `stars`, which star imports `stars.second`, which defines `Starred`.
class FromStarCycle(Starred):
    table_name = {}


# No module of the cycle gives out `Nowhere`.
class FromStarCycleMissing(Nowhere):
    table_name = {}  # expect: 5 [unused-attribute]


print(FromStarCycle, FromStarCycleMissing)
