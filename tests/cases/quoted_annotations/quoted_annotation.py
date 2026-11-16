# The names in a quoted annotation are read, as in one that is not quoted.
class OnlyQuoted:
    pass


class Unused:  # expect: 7 [unused-class]
    pass


def take(value: "OnlyQuoted") -> "list[int]":
    return [value]


# An annotation that is not a valid expression reads nothing.
def broken(value: "not valid ("):
    return value


take(1)
broken(1)
