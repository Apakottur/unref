# The parameter of a lambda is local to it.
value = 1  # expect: unused variable
other = 2

print(lambda value: value + other)
