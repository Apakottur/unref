def public():
    return 1


def also_public():
    return 2


def _private():  # expect: unused function
    return 3
