# The first parameter of a method of a metaclass is a class that it makes, which does not inherit
# from it, so its members are matched by name.
class Registry(type):
    def build(cls):
        return "registry"

    def create(cls):
        return cls.build()


class Widget(metaclass=Registry):
    @classmethod
    def build(cls):
        return "widget"


Widget.create()
