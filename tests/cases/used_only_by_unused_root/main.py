# What a function that counts as used uses is used: by the options, a dunder or an ignore comment.
def by_name():
    return from_by_name()


def from_by_name():
    return 1


@register
def by_decorator():
    return from_by_decorator()


def from_by_decorator():
    return 2


class Class:  # expect: 7 [unused-class]
    def __init__(self):
        self.value = from_dunder()  # expect: 14 [unused-attribute] transitive


def from_dunder():
    return 3


def by_comment():  # unref: ignore[unused-function]
    return from_by_comment()


def from_by_comment():
    return 4


# An ignore comment is needed when the definition is unused without it, even when it is used with it.
def ignored_cycle():  # unref: ignore[unused-function]
    return cycle()


def cycle():
    return ignored_cycle()
