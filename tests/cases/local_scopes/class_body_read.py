# A read in a class body of a name that the body has bound is a reference to that member of the class,
# and a read of a name that it has not bound yet is a reference to the name around the class.
limit = 10
size = 1
items = [1]  # expect: 1 [unused-variable]
step = 2
Inner = None


class Class:
    size = size + 1
    double = size * 2  # expect: 5 [unused-attribute]
    items = [1, 2]
    step = 3  # expect: 5 [unused-attribute]
    # The first iterable of a comprehension sees the names of the class body, the rest does not.
    pairs = [(item, other) for item in items for other in (limit, step)]  # expect: 5 [unused-attribute]

    class Inner:  # expect: 11 [unused-class]
        # A nested class body does not see the names of the class body that it is in.
        total = step  # expect: 9 [unused-attribute] transitive

    def method(self):
        # A method does not see the names of its class body either.
        return Inner

    other = method  # expect: 5 [unused-attribute]


Class()
