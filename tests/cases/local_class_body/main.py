# A read in a class body of a name of that body is a reference to the attribute.
class Class:
    base = 1
    derived = base + 1  # expect: unused attribute

    def method(self):  # expect: unused method
        # The names of a class body are not visible in its methods.
        return base


Class()
