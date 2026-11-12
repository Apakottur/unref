class Base:
    pass


def by_absolute():
    return 1


def by_attribute():
    return 2


def unused():  # expect: unused function
    return 3
