# Only names that both start and end with two underscores are dunders.
def __private():  # expect: unused function
    return 1


def _single_():  # expect: unused function
    return 2


def trailing__():  # expect: unused function
    return 3


__leading = 4  # expect: unused variable
