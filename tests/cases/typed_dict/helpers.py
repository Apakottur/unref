from typing import TypedDict


def send(value: object) -> None:
    value


# Read as a whole under the name that it is imported as.
class Aliased(TypedDict):
    aliased: int
