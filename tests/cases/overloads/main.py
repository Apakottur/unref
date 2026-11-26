import typing
from typing import Protocol, overload


@overload
def unused(value: int) -> int: ...
@overload
def unused(value: str) -> str: ...
def unused(value):  # expect: 5 [unused-function]
    return value


@typing.overload
def used(value: int) -> int: ...
@typing.overload
def used(value: str) -> str: ...
def used(value):
    return value


used(1)


@overload
def ignored(value: int) -> int: ...
@overload
def ignored(value: str) -> str: ...
def ignored(value):  # unref: ignore[unused-function]
    return value


@overload
def stubs_only(value: int) -> int: ...  # expect: 5 [unused-function]
@overload
def stubs_only(value: str) -> str: ...


def redefined():  # expect: 5 [unused-function]
    return 1


def redefined():  # expect: 5 [unused-function]
    return 2


class Reader(Protocol):
    @overload
    def read(self, size: int) -> bytes: ...  # expect: 9 [unused-method]
    @overload
    def read(self, size: None) -> str: ...


class File:
    @overload
    def write(self, data: bytes) -> int: ...
    @overload
    def write(self, data: str) -> int: ...
    def write(self, data):  # expect: 9 [unused-method]
        return len(data)


def use(reader: Reader) -> File:
    return reader and File()


use
