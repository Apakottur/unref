# A read in a class body of a name of that body is a reference to the attribute.
class Class:
    base = 1
    derived = base + 1  # expect: 5 [unused-attribute]

    def method(self):  # expect: 9 [unused-method]
        # The names of a class body are not visible in its methods.
        return base


Class()
