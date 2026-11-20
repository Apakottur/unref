from typing import Final, Literal

OrderStatus = Literal["PENDING", "SHIPPED", "CANCELLED"]


# Every member counts as used, though only one is read.
class OrderStatusValues:
    PENDING: Final = "PENDING"
    SHIPPED: Final = "SHIPPED"
    CANCELLED: Final = "CANCELLED"


# The pattern must match the whole name.
class ValuesOfColor:
    RED: Final = "RED"  # expect: 5 [unused-attribute]


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
