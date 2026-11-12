import registry
from registry import app, decorators, other, plain


@app.command()
def with_call():
    return 1


@app.command
def without_call():
    return 2


@app.command("name", help="text")
def with_arguments():
    return 3


# A class can be decorated too.
@registry.register
class Registered:
    pass


@plain
async def plain_name():
    return 4


# One of the decorators is enough.
@other
@app.command()
def stacked():
    return 5


class Class:  # expect: 7 [unused-class]
    @app.command()
    def method(self):
        return 6


@other
def not_listed():  # expect: 5 [unused-function]
    return 7


# The whole dotted name has to match.
@app.commands()
def other_name():  # expect: 5 [unused-function]
    return 8


@registry.app.command()
def longer_name():  # expect: 5 [unused-function]
    return 9


@command()
def shorter_name():  # expect: 5 [unused-function]
    return 10


# A decorator that is not a dotted name cannot be listed.
@decorators[0]
def subscript():  # expect: 5 [unused-function]
    return 11


@decorators[0].command
def attribute_of_subscript():  # expect: 5 [unused-function]
    return 12
