# A read in a class body of an attribute that a method assigns as well is a reference to the attribute,
# whether the method is above the class body assignment or below it.
class First:
    def __init__(self):
        self.size = 1

    size = 0
    doubled = size * 2


class Second:
    size = 0
    doubled = size * 2

    def __init__(self):
        self.size = 1


class Third:
    def __init__(self):
        self.size = 1  # expect: 14 [unused-attribute]

    size = 0


First().doubled
Second().doubled
Third()
