from typing import Final, Literal

Color = Literal["RED", "GREEN", "BLUE"]


# Every member counts as used, though only one is read.
class ColorValues:
    RED: Final = "RED"
    GREEN: Final = "GREEN"
    BLUE: Final = "BLUE"


# The override applies to everything in the class: attributes, methods and what is on the instance.
class ShapeValues:
    CIRCLE: Final = "CIRCLE"

    def method(self):
        self.on_instance = 1


# The pattern must match the whole name.
class ValuesOfColor:
    RED: Final = "RED"  # expect: 5 [unused-attribute]


# The override does not apply to a subclass whose name does not match.
class Palette(ColorValues):
    EXTRA: Final = "EXTRA"  # expect: 5 [unused-attribute]


# A nested class is matched by its own name, not the name of the class it is in.
class SizeValues:
    class Inner:
        SMALL: Final = "SMALL"  # expect: 9 [unused-attribute]


class Outer:
    class NestedValues:
        LARGE: Final = "LARGE"


# The class itself is not selected: an unused class with a matching name is still reported.
class UnusedValues:  # expect: 7 [unused-class]
    ITEM: Final = "ITEM"


# The override does not apply outside a class.
TopValues: Final = "TOP"  # expect: 1 [unused-variable]

print(Color, ColorValues.RED, ShapeValues, ValuesOfColor, Palette, SizeValues.Inner, Outer.NestedValues)
