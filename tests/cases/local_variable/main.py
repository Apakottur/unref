# Local variables are not reported, Ruff covers them.
def function():  # expect: unused function
    local = 1
    annotated: int = 2
    first, second = 3, 4
    return 5
