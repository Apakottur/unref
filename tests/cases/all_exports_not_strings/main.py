import os

# Only string literals are read from `__all__`.
__all__ = [os.name, 1, *os.sys.path]
__all__ = os.listdir()


def function():  # expect: 5 [unused-function]
    return 1
