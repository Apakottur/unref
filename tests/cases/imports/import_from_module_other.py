def function():  # expect: 5 [unused-function]
    return 1


class Class:  # expect: 7 [unused-class]
    pass


class Renamed:  # expect: 7 [unused-class]
    pass
