def outer():  # expect: unused function
    def inner():  # expect: unused function
        return 1


class Class:  # expect: unused class
    def method(self):  # expect: unused method
        def inner():  # expect: unused function
            return 2
