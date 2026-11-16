# The file does not parse, so its ignore comment is not reported as unused.
def function():  # unref: ignore[unused-function]
    pass


# The finding is where the parser stops, as with ruff and ty.
print "Python 2"  # expect: 7 [invalid-syntax]
