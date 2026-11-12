# A dotted object uses the class and its member.
class Plugin:
    def load(self):
        return 1

    def unload(self):  # expect: 9 [unused-method]
        return 2


# A member that cannot be found leaves the class used.
class Factory:
    def build(self):  # expect: 9 [unused-method]
        return 3
