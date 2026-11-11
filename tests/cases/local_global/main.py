# A name that is declared global is not local.
counter = 0
other = 1  # expect: unused variable


def function():
    global counter
    counter += 1
    other = 2
    return counter + other


function()
