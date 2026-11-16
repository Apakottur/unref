# `super()` reads from the base classes only.
class Base:
    def load(self):
        return 1

    def store(self):
        return 2


class Child(Base):
    def load(self):  # expect: 9 [unused-method]
        return super().load()

    def store(self):
        return super(Child, self).store()

    def reload(self):
        return super().load()


Child().reload()
Child().store()
