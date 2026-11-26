# Passing the namespace of the module on reads every name of it.
def index(): ...


class About: ...


repr(dict(vars()))


class Unread:
    # The names of the module are read, but not the members of its classes.
    def unused(self): ...  # expect: 9 [unused-method]
