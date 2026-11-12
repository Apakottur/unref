def in_renamed_submodule():
    return 1


def in_imported_submodule():  # expect: 5 [unused-function]
    return 2
