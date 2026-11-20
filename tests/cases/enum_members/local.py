from enum import Enum


def read_whole():
    # An enum in a function, read as a whole.
    class Color(Enum):
        RED = 1
        GREEN = 2

    return list(Color)


def read_members():
    # An enum in a function, read only as members.
    class Color(Enum):
        RED = 1
        GREEN = 2  # expect: 9 [unused-attribute]

    return Color.RED


def read_method():
    # A method read from an enum in a function, which can reach any member.
    class Color(Enum):
        RED = 1
        GREEN = 2

        @classmethod
        def parse(cls, text):
            return cls[text]

    return Color.parse("RED")


read_whole()
read_members()
read_method()
