# A function that only calls itself is unused: its body runs only if something else calls it.
def countdown(value):  # expect: 5 [unused-function]
    if value:
        countdown(value - 1)


# The same goes for functions that only call each other.
def ping(value):  # expect: 5 [unused-function]
    return pong(value - 1) if value else 0


def pong(value):  # expect: 5 [unused-function]
    return ping(value - 1) if value else 0


# A recursive function that something else calls is used.
def factorial(value):
    return value * factorial(value - 1) if value else 1


print(factorial(3))
