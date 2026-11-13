# Reading a module-level name in its own module uses it.
def function():
    return 1


class Class:
    pass


def key(item):
    return -item


LIMIT = 10
OTHER = 20  # expect: 1 [unused-variable]

function()
Class()
sorted([1, 2, 3], key=key)
print(LIMIT)
