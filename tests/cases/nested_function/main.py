def outer():  # expect: 5 [unused-function]
    def inner():  # expect: 9 [unused-function] transitive
        return 1


class Class:  # expect: 7 [unused-class]
    def method(self):  # expect: 9 [unused-method] transitive
        def inner():  # expect: 13 [unused-function] transitive
            return 2
