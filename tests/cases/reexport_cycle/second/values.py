from enum import Enum


class Shade(Enum):
    DARK = 1
    LIGHT = 2  # expect: 5 [unused-attribute]


def unused():  # expect: 5 [unused-function]
    return 1
