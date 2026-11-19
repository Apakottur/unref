import json
import logging
from json import dumps
from typing import TypedDict

import helpers
from helpers import Aliased as Renamed


# A TypedDict that is read as a whole has all its fields used, and those of the TypedDicts it inherits
# from: `{**value}`, `f(**value)`, a loop, `value.items()`, or passing it to a function from outside
# the project, when the value is the result of a call, or a local variable that is assigned one.
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


class Logged(TypedDict):
    logged: int


class Printed(TypedDict):
    printed: int


# Passing the value to a function of the project or to a method of another object reads no key.
class Passed(TypedDict):
    passed: int  # expect: 5 [unused-attribute]
    imported: int  # expect: 5 [unused-attribute]
    sent: int  # expect: 5 [unused-attribute]


# Read as a whole only in a function that is unused.
class Unused(TypedDict):  # expect: 7 [unused-class] transitive
    unused_field: int  # expect: 5 [unused-attribute] transitive


# A value that is not the result of a call, or a variable of the module, is not known.
class Unknown(TypedDict):
    unknown: int  # expect: 5 [unused-attribute]


UNKNOWN = Unknown(unknown=1)


def keywords(keyword: int) -> int:
    return keyword


def take(value: object) -> None:
    value


class Client:
    def send(self, value: object) -> None:
        value


def read(client: Client) -> None:
    event = RetryEvent(attempt=1, delay_ms=250)
    json.dumps(dict(event))
    spread = Spread(spread=1)
    {**spread}
    keywords(**Keywords(keyword=1))
    looped = Looped(looped=1)
    for key in looped:
        key
    comprehended = Comprehended(comprehended=1)
    [key for key in comprehended]
    Items(item=1).items()
    dumps(obj=Dumped(dumped=1))
    logging.info("%s", Logged(logged=1))
    printed = Printed(printed=1)
    # `print` is from outside the project.
    print(printed)  # noqa: T201
    take(Passed(passed=1))
    helpers.send(Passed(imported=1))
    client.send(Passed(sent=1))
    dict(UNKNOWN)
    dict(Renamed(aliased=1))


def unused() -> None:  # expect: 5 [unused-function]
    dict(Unused(unused_field=1))


read(Client())
