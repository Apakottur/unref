def used_by_fixtures():  # expect: unused function
    return 1


def used_by_node_modules():
    return 1


def used_by_script():  # expect: unused function
    return 1


def used_by_kept_script():
    return 1
