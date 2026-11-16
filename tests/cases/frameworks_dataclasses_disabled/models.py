from dataclasses import dataclass
from typing import NamedTuple


# The rules of dataclasses are turned off.
@dataclass
class Point:
    x: int  # expect: 5 [unused-attribute]
    y: int = 0  # expect: 5 [unused-attribute]


class Pair(NamedTuple):
    first: int  # expect: 5 [unused-attribute]


print(Point, Pair)
