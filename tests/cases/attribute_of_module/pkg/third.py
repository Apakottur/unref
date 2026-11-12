def in_imported_submodule():
    return 1


def in_renamed_submodule():  # expect: unused function
    return 2
