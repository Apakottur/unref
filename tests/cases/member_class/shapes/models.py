class Shape:
    def describe(self):
        return "shape"

    def area(self):
        return 0


class Square(Shape):
    sides = 4

    def describe(self):  # expect: 9 [unused-method]
        return "square"

    def area(self):
        return 1


class Circle:
    sides = 0  # expect: 5 [unused-attribute]

    def area(self):  # expect: 9 [unused-method]
        return 2
