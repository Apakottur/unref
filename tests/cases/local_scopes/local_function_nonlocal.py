# A name that is declared nonlocal is the variable of the enclosing function, not a local variable
# and not a name of the module.
def helper():  # expect: 5 [unused-function]
    return 0


def read_only():
    def helper():
        return 1

    def inner():
        nonlocal helper
        return helper()

    return inner()


def rebind():
    def helper():
        return 2

    def inner():
        nonlocal helper
        previous = helper
        helper = None
        return previous()

    return inner()


def redefine():
    def helper():
        return 3

    def inner():
        nonlocal helper

        def helper():
            return 4

    inner()
    return helper()


def redefine_nested():
    def helper():
        return 5

    def middle():
        def inner():
            nonlocal helper

            def helper():
                return 6

        inner()

    middle()
    return helper()


read_only(), rebind(), redefine(), redefine_nested()
