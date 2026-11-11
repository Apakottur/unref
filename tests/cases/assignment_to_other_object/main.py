import types

namespace = types.SimpleNamespace()  # expect: unused variable
items = {}  # expect: unused variable

# Not definitions: they change an object that is defined elsewhere.
namespace.attribute = 1
items["key"] = 2
namespace.first, items["second"] = 3, 4


class Class:  # expect: unused class
    def method(self, other):  # expect: unused method
        other.attribute = 5
        self.items = {}  # expect: unused attribute
        self.items["key"] = 6
