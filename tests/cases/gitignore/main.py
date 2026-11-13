def used_by_ignored():  # expect: 5 [unused-function]
    return 1


def used_by_kept():
    return 1


def used_by_nested():
    return 1
