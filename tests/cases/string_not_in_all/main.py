# A string is a reference only in `__all__`.
NAMES = ["function"]
other = ["function"]  # expect: unused variable

print(NAMES)


def function():  # expect: unused function
    return 1
