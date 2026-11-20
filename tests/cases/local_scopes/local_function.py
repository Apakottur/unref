# A function or class defined in a function is local to it, so a read of its name there is not a
# reference to something of the same name.
def helper():  # expect: 5 [unused-function]
    return 1


class Box:  # expect: 7 [unused-class]
    pass


def run():
    def helper():
        return 2

    class Box:
        pass

    return helper(), Box()


run()


# A read of the name elsewhere is not a reference to the local function or class.
def used():
    return 3


class Used:
    pass


def other():
    def used():  # expect: 9 [unused-function]
        return 4

    class Used:  # expect: 11 [unused-class]
        pass

    return 5


used(), Used(), other()
