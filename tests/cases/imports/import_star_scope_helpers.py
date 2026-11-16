# A star import only binds the names of the module, not the methods and attributes of its classes
# or the functions and classes defined in its functions.
class Shape:
    sides = 0  # expect: 5 [unused-attribute]

    def area(self):  # expect: 9 [unused-method]
        return 0


def build():
    def inner():  # expect: 9 [unused-function]
        return 1

    class Local:  # expect: 11 [unused-class]
        pass

    return Shape()


def install():
    global installed

    def installed():
        return 2
