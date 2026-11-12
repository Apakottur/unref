def in_package():
    return 1


def in_submodule():  # expect: 5 [unused-function]
    return 2
