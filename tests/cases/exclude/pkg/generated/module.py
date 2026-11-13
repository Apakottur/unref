# Only `generated` at the top of the project is excluded, not every directory of that name.
def nested():  # expect: 5 [unused-function]
    return 1
