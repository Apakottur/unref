# The target of a walrus defines a variable, or an attribute in a class body.
if (unused := 1):  # expect: 5 [unused-variable]
    pass

if (used := 2):
    pass


class Holder:
    (member := 3)  # expect: 6 [unused-attribute]


# The target of a walrus in a comprehension is not collected.
[(kept := value) for value in range(3)]


def local():
    # A local variable is left to Ruff.
    if (found := 4):
        pass


used, Holder, local
