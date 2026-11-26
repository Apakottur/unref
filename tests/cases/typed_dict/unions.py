from typing import Optional, TypeAlias, TypedDict, Union


class Request(TypedDict):
    request: int


class Response(TypedDict):
    response: int


class Error(TypedDict):
    error: int


class Notice(TypedDict):
    notice: int


class Event(TypedDict):
    event: int


# A union alias that is only read by a key reads no other key.
class Keyed(TypedDict):
    keyed: int
    other: int  # expect: 5 [unused-attribute]


Message = Request | Response
Failure = Union[Error, None]
Alert: TypeAlias = Optional[Notice]
type Signal = Event
KeyedAlias = Keyed | None
