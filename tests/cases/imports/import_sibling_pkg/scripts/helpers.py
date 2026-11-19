# A module that the script imports runs with the same import path.
from formatting import prefix


def warn(message):
    return prefix(message)


def unused():  # expect: 5 [unused-function]
    return 1
