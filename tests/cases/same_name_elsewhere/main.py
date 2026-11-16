# An attribute read from an object whose class is not known is matched by name only, so it keeps
# every method and attribute of that name alive.
class First:
    def method(self):
        return 1


class Second:
    def method(self):
        return 2


def call(value):
    return value.method()


call(First())
Second()
