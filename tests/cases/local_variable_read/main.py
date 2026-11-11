# A read of a local variable is not a reference to something of the same name.
def helper():  # expect: unused function
    return 1


total: int = 2  # expect: unused variable


def function():
    helper = 3
    total: int = 4
    return helper + total


function()
