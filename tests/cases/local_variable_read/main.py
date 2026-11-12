# A read of a local variable is not a reference to something of the same name.
def helper():  # expect: 5 [unused-function]
    return 1


total: int = 2  # expect: 1 [unused-variable]


def function():
    helper = 3
    total: int = 4
    return helper + total


function()
