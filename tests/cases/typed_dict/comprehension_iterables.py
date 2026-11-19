from typing import TypedDict


class Packet(TypedDict):
    secret: int  # expect: 5 [unused-attribute]


class Envelope(TypedDict):
    payload: int


def read():
    packet = Packet(secret=1)
    envelope = Envelope(payload=2)
    # A later iterable sees the earlier iteration target.
    keys = [key for packet in ({"other": 2},) for key in packet]
    # Without a shadowing target, a later iterable still reads the outer value.
    return keys, [key for _ in range(1) for key in envelope]


print(read())
