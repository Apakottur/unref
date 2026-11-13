from enum import Enum


class Color(Enum):
    RED = 1
    GREEN = 2  # expect: 5 [unused-attribute]
