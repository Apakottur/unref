from collections import namedtuple


class Future:
    _value = None

    def result(self):
        return self._value


# A class whose base class is a variable or a call is not known to inherit from anything, so its
# members are matched by name in every class.
_Base = Future


class Task(_Base):
    def __init__(self):
        self._value = 1


class Point(namedtuple("Point", "x y")):
    def result(self):
        return self.x


Task().result()
Point(1, 2)
