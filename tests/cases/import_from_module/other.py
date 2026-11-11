def function():  # expect: unused function
    return 1


class Class:  # expect: unused class
    pass


class Renamed:  # expect: unused class
    pass
