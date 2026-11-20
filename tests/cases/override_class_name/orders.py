from statuses import OrderStatus, OrderStatusValues, Outer, SizeValues, ValuesOfColor


def is_open(status: OrderStatus) -> bool:
    return status == OrderStatusValues.PENDING


print(is_open("SHIPPED"), ValuesOfColor, SizeValues.Inner, Outer.NestedValues)
