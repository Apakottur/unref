from typing import TypedDict

from . import source
from . import replaced_source


class Replacement:
    def value(self):
        return 9


replaced_source = Replacement()


class Packet(TypedDict):
    key: int


def read():
    packet = Packet(key=1)
    return dict(packet)


read()
TypedDict = "replacement"
TypedDict
