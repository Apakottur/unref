import enum
from enum import Enum, Flag, IntEnum, StrEnum


# Read only as members: a member that is not read is unused.
class Status(Enum):
    ACTIVE = 1
    INACTIVE = 2  # expect: 5 [unused-attribute]

    # Methods are reported as usual.
    def describe(self):  # expect: 9 [unused-method]
        return self.name


# Only a type hint of a parameter or a return, which reads no member.
class Level(IntEnum):
    LOW = 1
    HIGH = 2  # expect: 5 [unused-attribute]


# Read as a whole, which can reach any member: by value, by iteration, by name, and as the type of a
# field, which a library such as Pydantic or SQLAlchemy converts values into.
class Color(Enum):
    RED = 1
    GREEN = 2


class Size(StrEnum):
    SMALL = "s"
    LARGE = "l"


class Mode(enum.Enum):
    READ = "r"
    WRITE = "w"


class Shade(Flag):
    DARK = 1
    LIGHT = 2


# A class method can read any member, through `cls`.
class Priority(Enum):
    LOW = 1
    HIGH = 2

    @classmethod
    def parse(cls, text):
        return cls[text.upper()]


class Weekday(Enum):
    MONDAY = 1
    TUESDAY = 2


# Imported by another module, and read there.
class Direction(Enum):
    UP = 1
    DOWN = 2


class Axis(Enum):
    X = 1
    Y = 2  # expect: 5 [unused-attribute]


# Read as a whole only by an unused function.
class Unit(Enum):
    METER = 1
    FOOT = 2  # expect: 5 [unused-attribute] transitive


# Not an enum: a class attribute is read by its name.
class Plain:
    RED = 1
    BLUE = 2  # expect: 5 [unused-attribute]


class Point:
    shade: Shade  # expect: 5 [unused-attribute]


def paint(level: Level) -> Level:
    return Level.LOW


def unused():  # expect: 5 [unused-function]
    return list(Unit)


print(Status.ACTIVE, Color(1), list(Size), Mode["READ"], Point, Priority.parse("low"), Weekday.__members__)
print(paint, Unit.METER, Plain.RED)
