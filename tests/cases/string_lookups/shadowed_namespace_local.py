BY_PARAMETER = 0  # expect: 1 [unused-variable]
BY_CLOSURE = 0  # expect: 1 [unused-variable]
BY_BUILTIN = 4
AFTER_CONDITIONAL = 5
INSIDE_ELSE = 6
EXPLICIT_MODULE_BUILTIN = 7
EXPLICIT_LOCAL_BUILTIN = 8


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

if False:
    globals = lambda: {"AFTER_CONDITIONAL": 0}
else:
    globals()["INSIDE_ELSE"]

globals()["AFTER_CONDITIONAL"]

from builtins import globals

globals()["EXPLICIT_MODULE_BUILTIN"]


def explicit_import():
    from builtins import globals

    return globals()["EXPLICIT_LOCAL_BUILTIN"]


explicit_import()
