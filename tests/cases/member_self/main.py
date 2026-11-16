# A member read from the first parameter of a method is matched in its class.
class Reader:
    def read(self):
        return self.parse()

    def parse(self):
        return 1

    @classmethod
    def create(cls):
        return cls.default()

    @classmethod
    def default(cls):
        return cls()


class Writer:
    def parse(self):  # expect: 9 [unused-method]
        return 2

    def default(self):  # expect: 9 [unused-method]
        return 3


Reader().read()
Reader.create()
Writer()
