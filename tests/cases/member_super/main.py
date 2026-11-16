# `super()` reads from the base classes only.
class Base:
    def load(self):
        return 1

    def save(self):
        return 2


class Child(Base):
    def load(self):  # expect: 9 [unused-method]
        return super().load()

    def save(self):
        return super(Child, self).save()

    def run(self):
        return super().load()


Child().run()
Child().save()
