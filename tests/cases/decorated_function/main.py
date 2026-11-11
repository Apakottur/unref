import functools


@functools.cache
def function():  # expect: unused function
    return 1
