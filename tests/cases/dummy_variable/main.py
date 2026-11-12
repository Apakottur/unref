# A variable whose name matches `dummy-variable-rgx` is not reported where Python forces a binding: as a
# target of an unpacking, a for loop or a comprehension. The default pattern is the one of Ruff.


def pair():
    return 1, 2


key, _public_key = pair()
_first, _second = pair()
head, *_rest = [3, 4, 5]
outer, (_inner, *_others) = 6, (7, 8)
[listed, _listed] = pair()
__, ___many = pair()
print(key, head, outer, listed)

for _attempt in range(3):
    print("attempt")

for _index, row in enumerate([9]):
    print(row)

print([value for _position, value in enumerate([10])])
print({value for (_place, value) in enumerate([11])})


class Holder:
    left, _right = pair()

    def __init__(self):
        # An attribute of an object is not a dummy variable: `self._kept` can be read elsewhere.
        self.kept, self._kept = pair()  # expect: unused attribute, unused attribute


print(Holder().left)


# Where the binding can be dropped, the name is reported.
_unused = 12  # expect: unused variable
_response = pair()  # expect: unused variable
_annotated: int = 13  # expect: unused variable
_chained = _chained_too = 14  # expect: unused variable, unused variable


class Settings:  # expect: unused class
    _private = 15  # expect: unused attribute


# A name that does not match the pattern is reported in an unpacking.
used, _trailing_ = pair()  # expect: unused variable
print(used)


# In a function, every shape is the same: local variables are not reported.
async def function(items, stream):  # expect: unused function
    used, _unused = pair()
    _a, _b = pair()
    for _attempt in range(3):
        print(used)
    async for _item, value in stream:
        print(value)
    print([value for _index, value in items])
    _plain = 16
