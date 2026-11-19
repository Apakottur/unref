# A string literal looked up in the namespace of the module reads the name from the module.
BY_GLOBALS = 1
BY_GET = 2
BY_MEMBERSHIP = 3
BY_VARS = 4
BY_LOCALS = 5
IN_FUNCTION = 6
HIDDEN_BY_LOCAL = 7


def handler():
    return 8


print(
    globals()["BY_GLOBALS"],
    globals().get("BY_GET"),
    "BY_MEMBERSHIP" in globals(),
    vars()["BY_VARS"],
    locals()["BY_LOCALS"],
    globals()["handler"](),
)


def read_in_function():
    # The namespace of the module holds no local variable.
    HIDDEN_BY_LOCAL = 9
    return globals()["IN_FUNCTION"], globals()["HIDDEN_BY_LOCAL"], HIDDEN_BY_LOCAL


read_in_function()


# Assigning a name in the namespace does not read it.
WRITTEN = 10  # expect: 1 [unused-variable]
globals()["WRITTEN"] = 11

# A name built when the code runs is not known.
COMPUTED = 12  # expect: 1 [unused-variable]
SUFFIX = "PUTED"
print(globals()["COM" + SUFFIX])

# Inside a function, `vars()` and `locals()` hold its local variables, not the variables of the module.
BY_LOCAL_VARS = 13  # expect: 1 [unused-variable]
BY_LOCAL_LOCALS = 14  # expect: 1 [unused-variable]


def read_locals():
    return vars()["BY_LOCAL_VARS"], locals()["BY_LOCAL_LOCALS"]


read_locals()

# Code in a string is not read.
BY_EVAL = 15  # expect: 1 [unused-variable]
print(eval("BY_EVAL"))
