class Class:
    def method(self):
        return 1

    def other(self):  # expect: 9 [unused-method]
        return 2


Class().method()
