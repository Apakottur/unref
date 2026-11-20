from typing import TypedDict


class Packet(TypedDict):
    secret: int


packet = Packet(secret=1)
for key in packet:
    key


class Bundle(TypedDict):
    payload: int


bundle: Bundle = Bundle(payload=1)
[key for key in bundle]


class Rebound(TypedDict):
    hidden: int  # expect: 5 [unused-attribute]


rebound = Rebound(hidden=1)
rebound = {"other": 1}
list(rebound)
