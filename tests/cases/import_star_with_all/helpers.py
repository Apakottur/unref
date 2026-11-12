# With `__all__`, a star import only imports the names in it.
__all__ = ["listed"]


def listed():
    return 1


def unlisted():  # expect: 5 [unused-function]
    return 2
