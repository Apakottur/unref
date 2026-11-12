from dataclasses import KW_ONLY, dataclass
from functools import singledispatch

# `_` is the name for a value that is not used, however it is bound.
_ = print("side effect")
_: int = len("annotated")
first, _ = 1, 2  # expect: unused variable
*_, last = 1, 2, 3  # expect: unused variable
for _ in range(2):
    print("loop")
with open(__file__) as _:
    print("with")


@dataclass
class Point:
    _: KW_ONLY
    x: int  # expect: unused attribute


class Holder:
    _ = "class body"

    def __init__(self):
        self._ = 1


@singledispatch
def show(value):
    print(value)


@show.register
def _(value: int):
    print(value)


Holder()
show(1)
print(Point(x=1))
