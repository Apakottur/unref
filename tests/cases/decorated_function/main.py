import functools


@functools.cache
def function():  # expect: 5 [unused-function]
    return 1
