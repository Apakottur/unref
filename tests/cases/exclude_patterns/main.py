def used_by_fixtures():  # expect: 5 [unused-function]
    return 1


def used_by_node_modules():
    return 1


def used_by_script():  # expect: 5 [unused-function]
    return 1


def used_by_kept_script():
    return 1
