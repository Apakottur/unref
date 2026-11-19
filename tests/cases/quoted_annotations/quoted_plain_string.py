# A string that is not in a type reads nothing, even when it is the name of a class.
class Plain:  # expect: 7 [unused-class]
    pass


class Default:  # expect: 7 [unused-class]
    pass


def take(value: int = "Default") -> None:  # type: ignore[assignment]
    "Plain", value


take()
