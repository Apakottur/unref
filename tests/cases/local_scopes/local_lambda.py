# The parameter of a lambda is local to it.
value = 1  # expect: 1 [unused-variable]
other = 2

lambda value: value + other
