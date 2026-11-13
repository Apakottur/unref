import typing
from typing import NotRequired, TypedDict

import typing_extensions


# A field of a TypedDict is used when a key with its name is read, anywhere in the project.
class Movie(TypedDict):
    title: str
    year: NotRequired[int]
    rating: float
    studio: str
    budget: int
    country: str
    language: str
    # Only written.
    director: str  # expect: 5 [unused-attribute]
    # Only read as an attribute.
    genre: str  # expect: 5 [unused-attribute]
    # Only read in a function that is unused.
    sequel: str  # expect: 5 [unused-attribute] transitive


class Options(typing.TypedDict, total=False):
    verbose: bool
    quiet: bool  # expect: 5 [unused-attribute]


class Point(typing_extensions.TypedDict):
    x: int


# Through a TypedDict of the project.
class Film(Movie):
    runtime: int


# A key read does not use an attribute of another class.
class Book:
    title = ""  # expect: 5 [unused-attribute]
    author = ""
