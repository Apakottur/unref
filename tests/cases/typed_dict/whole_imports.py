from typing import TypedDict


# A function from outside the project reads a TypedDict as a whole when the module imports it after
# the function, or the function imports it.
class Late(TypedDict):
    late: str


class Local(TypedDict):
    local: str


# A function-local import of a module of the project reads no key.
class Helped(TypedDict):
    helped: str  # expect: 5 [unused-attribute]


def late_import() -> str:
    late = Late(late="a")
    return json.dumps(late)


def local_import() -> str:
    import json as j

    local = Local(local="a")
    return j.dumps(local)


def local_project_import() -> None:
    import helpers as h

    h.send(Helped(helped="a"))


import json

late_import(), local_import(), local_project_import()
