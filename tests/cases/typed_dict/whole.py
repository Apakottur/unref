import json
from json import dumps
from typing import TypedDict


# A TypedDict that is read as a whole has all its fields used, and those of the TypedDicts it inherits
# from: `dict(value)`, `json.dumps(value)`, `{**value}`, `f(**value)`, a loop, or `value.items()`, when
# the value is the result of a call, or a variable that is assigned one.
class Retry(TypedDict):
    attempt: int


class RetryEvent(Retry):
    delay_ms: int


class Spread(TypedDict):
    spread: int


class Keywords(TypedDict):
    keyword: int


class Looped(TypedDict):
    looped: int


class Comprehended(TypedDict):
    comprehended: int


class Items(TypedDict):
    item: int


class Dumped(TypedDict):
    dumped: int


class Global(TypedDict):
    global_field: int


# Passing the value to a function of the project or to `print` reads no key.
class Passed(TypedDict):
    passed: int  # expect: 5 [unused-attribute]
    printed: int  # expect: 5 [unused-attribute]


# Read as a whole only in a function that is unused.
class Unused(TypedDict):  # expect: 7 [unused-class] transitive
    unused_field: int  # expect: 5 [unused-attribute] transitive


# A parameter is not the variable of the module of the same name.
class Shadowed(TypedDict):
    shadowed: int  # expect: 5 [unused-attribute]


GLOBAL = Global(global_field=1)
shadowed = Shadowed(shadowed=1)


def keywords(keyword: int) -> int:
    return keyword


def take(value: object) -> None:
    print(value)


def consume(shadowed: dict[str, int]) -> None:
    print(dict(shadowed))


def read() -> None:
    event = RetryEvent(attempt=1, delay_ms=250)
    print(json.dumps(dict(event)))
    spread = Spread(spread=1)
    print({**spread})
    print(keywords(**Keywords(keyword=1)))
    looped = Looped(looped=1)
    for key in looped:
        print(key)
    comprehended = Comprehended(comprehended=1)
    print([key for key in comprehended])
    print(Items(item=1).items())
    print(dumps(Dumped(dumped=1)))
    print(list(GLOBAL))
    take(Passed(passed=1))
    printed = Passed(printed=1)
    print(printed)


def unused() -> None:  # expect: 5 [unused-function]
    print(dict(Unused(unused_field=1)))


read()
consume(shadowed)
