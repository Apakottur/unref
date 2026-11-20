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


# An import of a function around the call counts, also for a call in a comprehension, unless a scope in
# between imports the name again.
class Enclosed(TypedDict):
    enclosed: str


class Comprehended(TypedDict):
    comprehended: str


class Hidden(TypedDict):
    hidden: str  # expect: 5 [unused-attribute]


def enclosing_import() -> str:
    import json as outer_json

    def nested() -> str:
        return outer_json.dumps(Enclosed(enclosed="a"))

    return nested()


def comprehension_import() -> list[str]:
    import json as each_json

    return [each_json.dumps(Comprehended(comprehended="a")) for _ in range(2)]


def hidden_import() -> None:
    import json as hidden

    def nested() -> None:
        import helpers as hidden

        hidden.send(Hidden(hidden="a"))

    nested()


import json

late_import(), local_import(), local_project_import()
enclosing_import(), comprehension_import(), hidden_import()
