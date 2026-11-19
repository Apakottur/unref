def prefix(message):
    return "warning: " + message


def unused():  # expect: 5 [unused-function]
    return 1
