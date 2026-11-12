def public():
    return 1


def also_public():
    return 2


def _private():  # expect: 5 [unused-function]
    return 3
