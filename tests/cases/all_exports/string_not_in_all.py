# A string is a reference only in `__all__`.
NAMES = ["function"]
other = ["function"]  # expect: 1 [unused-variable]

print(NAMES)


def function():  # expect: 5 [unused-function]
    return 1
