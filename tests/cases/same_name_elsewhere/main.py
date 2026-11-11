# References are matched by name only, so a reference to one definition keeps every definition of
# that name alive.
class First:
    def method(self):
        return 1


class Second:
    def method(self):
        return 2


First().method()
Second()
