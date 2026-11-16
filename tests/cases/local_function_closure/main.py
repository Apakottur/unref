# A function or class of an enclosing function can be read from a nested function.
def helper():  # expect: 5 [unused-function]
    return 1


class Box:  # expect: 7 [unused-class]
    pass


def outer():
    def helper():
        return 2

    class Box:
        pass

    def middle():
        def inner():
            return helper(), Box()

        return inner()

    return middle()


outer()


# A local function or class that is read only from a nested function that is unused is unused.
def unused_outer():
    def unused_helper():  # expect: 9 [unused-function] transitive
        return 3

    def unused_inner():  # expect: 9 [unused-function]
        return unused_helper()

    return 4


unused_outer()
