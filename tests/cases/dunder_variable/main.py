# Python and tools read dunder variables by name.
__version__ = "1.0.0"
__author__: str = "author"


class Class:  # expect: unused class
    __slots__ = ("value",)
    __match_args__ = ("value",)
