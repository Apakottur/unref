def tool():
    return 1


def unused():  # expect: 5 [unused-function]
    return 2
