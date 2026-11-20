BY_PARAMETER = 0  # expect: 1 [unused-variable]
BY_CLOSURE = 0  # expect: 1 [unused-variable]
BY_BUILTIN = 4


def parameter(globals):
    return globals()["BY_PARAMETER"]


def enclosing():
    def read():
        return globals().get("BY_CLOSURE")

    globals = lambda: {"BY_CLOSURE": 1}
    return read()


def builtin():
    return globals()["BY_BUILTIN"]


parameter(lambda: {"BY_PARAMETER": 2}), enclosing(), builtin()
