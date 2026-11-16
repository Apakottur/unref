# A type alias statement defines a variable, or an attribute in a class body. What its value reads is
# read, although it is evaluated only when it is used.
type Unused = int  # expect: 6 [unused-variable]
type Used = list[Item]
type Generic[T] = dict[str, T]


class Item:
    type Member = int  # expect: 10 [unused-attribute]


Bound = int


# A bound of a type parameter of a function is read.
def generic[T: Bound](value: T) -> T:
    return value


def local():
    # A type alias in a function is a local variable, which is left to Ruff.
    type Local = int
    return 1


print(Used, Generic, generic, local)
