# Attributes are matched by name only, so a read of one attribute keeps every method and attribute of
# that name alive.
class First:
    def method(self):
        return 1


class Second:
    def method(self):
        return 2


First().method()
Second()
