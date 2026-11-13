# The file does not parse, so its ignore comment is not reported as unused.
def function():  # unref: ignore[unused-function]
    pass


print "Python 2"  # expect: 1 [invalid-syntax]
