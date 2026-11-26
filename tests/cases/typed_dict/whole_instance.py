from typing import Any, TypedDict


# A TypedDict that an attribute of the instance holds is read whole where a method reads the
# attribute whole: assigned a call of it in a method, annotated in a method or in the class body.
class Called(TypedDict):
    backend: str
    backend_options: dict[str, Any]


class Annotated(TypedDict):
    annotated: int


class InBody(TypedDict):
    in_body: int


class Parameter(TypedDict):
    parameter: int


# An attribute read whole in another class, or read through another object, reads no key.
class Elsewhere(TypedDict):
    elsewhere: int  # expect: 5 [unused-attribute]


class Other(TypedDict):
    other: int  # expect: 5 [unused-attribute]


def start(**kwargs: Any) -> dict[str, Any]:
    return kwargs


class Client:
    in_body: InBody

    def open(self) -> None:
        start(**self.called)
        {**self.annotated}, dict(self.in_body), [key for key in self.parameter]

    def __init__(self, parameter: Parameter) -> None:
        self.called = Called(backend="asyncio", backend_options={})
        self.annotated: Annotated = make()
        self.parameter = parameter
        self.elsewhere = Elsewhere(elsewhere=1)
        self.other = Other(other=1)


class Reader:
    def read(self, client: Client) -> None:
        {**self.elsewhere}, {**client.other}


def make() -> Any:
    return {}


Client(Parameter(parameter=1)).open()
Reader().read(Client(Parameter(parameter=1)))
