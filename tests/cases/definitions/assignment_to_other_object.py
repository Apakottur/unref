import types

# Not definitions: they change an object that is defined elsewhere.
types.attribute = 1
types.registry["key"] = 2
types.first, types.registry["second"] = 3, 4


class Class:  # expect: 7 [unused-class]
    def method(self, other):  # expect: 9 [unused-method] transitive
        other.attribute = 5
        other.registry["key"] = 6
