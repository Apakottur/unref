def function():  # expect: 5 [unused-function]
    return 1


# A reference from an excluded file does not count.
def used_by_excluded():  # expect: 5 [unused-function]
    return 2
