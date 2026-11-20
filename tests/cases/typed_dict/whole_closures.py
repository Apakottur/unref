from typing import TypedDict


class Packet(TypedDict):
    secret: int


class Deferred(TypedDict):
    deferred: int


class Shadowed(TypedDict):
    hidden: int  # expect: 5 [unused-attribute]


def read():
    packet = Packet(secret=1)

    def collect():
        for key in packet:
            key

    collect()


def read_deferred():
    def collect():
        return [key for key in packet]

    packet = Deferred(deferred=1)
    return collect()


def read_shadowed():
    packet = Shadowed(hidden=1)

    def collect():
        packet = {"other": 2}
        return list(packet)

    return packet, collect()


read(), read_deferred(), read_shadowed()
