from dataclasses import KW_ONLY, dataclass

# `_` is the name for a value that is not used.
_ = print("side effect")


@dataclass
class Point:
    _: KW_ONLY
    x: int  # expect: unused attribute


class Holder:
    def __init__(self):
        self._ = 1


Holder()
print(Point(x=1))
