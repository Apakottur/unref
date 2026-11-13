from enum import Enum


class Fruit(Enum):
    APPLE = 1
    PEAR = 2


class Berry(Enum):
    STRAWBERRY = 1
    RASPBERRY = 2  # expect: 5 [unused-attribute]
