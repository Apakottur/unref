# The variable of a comprehension is local to it.
item = 1  # expect: 1 [unused-variable]
key = 2  # expect: 1 [unused-variable]
element = 3  # expect: 1 [unused-variable]
member = 4  # expect: 1 [unused-variable]
ITEMS = [5, 6]

print([item for item in ITEMS])
print({key: key for key in ITEMS})
print({element for element in ITEMS})
print(list(member for member in ITEMS))

# Iteration targets do not shadow names in the containing function.
value = 7
other = 8


def read_globals():
    print([value for value in ITEMS])
    print({other for _, other in [(1, 2)]})
    return value, other


# An assignment expression in a nested comprehension binds in the containing function.
last = 0  # expect: 1 [unused-variable]


def read_local():
    print([[(last := number) for number in ITEMS] for _ in ITEMS])
    return last


print(read_globals(), read_local())
