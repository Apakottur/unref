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
