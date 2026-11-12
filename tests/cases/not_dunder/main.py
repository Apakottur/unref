# Only names that both start and end with two underscores are dunders.
def __private():  # expect: 5 [unused-function]
    return 1


def _single_():  # expect: 5 [unused-function]
    return 2


def trailing__():  # expect: 5 [unused-function]
    return 3


__leading = 4  # expect: 1 [unused-variable]
