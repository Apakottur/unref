# The target of `with ... as target` defines a variable, or an attribute in a class body.
from contextlib import nullcontext

with nullcontext() as unused:  # expect: 23 [unused-variable]
    pass

with nullcontext() as used:
    pass

# An unpacked name that matches `dummy-variable-rgx` is not reported.
with nullcontext((1, 2)) as (first, _second):  # expect: 30 [unused-variable]
    pass


class Holder:
    with nullcontext() as member:  # expect: 27 [unused-attribute]
        pass


def local():
    # A local variable is left to Ruff.
    with nullcontext() as handle:
        pass


used, Holder, local
