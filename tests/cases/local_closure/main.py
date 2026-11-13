# A read of a variable of an enclosing function is not a reference to something of the same name.
value = 1  # expect: 1 [unused-variable]


def outer():
    value = 2

    def inner():
        return value

    return inner


outer()
