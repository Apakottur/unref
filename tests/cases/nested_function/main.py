def outer():  # expect: unused function
    def inner():  # expect: unused function
        return 1

    return inner


class Class:  # expect: unused class
    def method(self):  # expect: unused method
        def inner():  # expect: unused function
            return 2

        return inner
