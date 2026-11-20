from typing import TypedDict


class Packet(TypedDict):
    secret: int  # expect: 5 [unused-attribute]


class Envelope(TypedDict):
    payload: int  # expect: 5 [unused-attribute]


class Container(TypedDict):
    field: int  # expect: 5 [unused-attribute]


class Live(TypedDict):
    count: int


def other():
    return {"other": 2}


def read(Packet):
    return list(Packet())


def assigned(Envelope):
    value = Envelope()
    return list(value)


def comprehension():
    return [key for Container in (other,) for key in Container()]


def builtin():
    return list(Live(count=1))


Packet(secret=1), Envelope(payload=1), Container(field=1)
read(other), assigned(other), comprehension(), builtin()
