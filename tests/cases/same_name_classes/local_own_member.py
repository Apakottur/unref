# `self.helper()` reads the method of its own class, not of a class of that name in another function.
def first():
    class Dummy:
        def helper(self):
            pass

        def run(self):
            self.helper()

    return Dummy().run()


def second():
    class Dummy:
        def helper(self):  # expect: 13 [unused-method]
            pass

    return Dummy()


first(), second()
