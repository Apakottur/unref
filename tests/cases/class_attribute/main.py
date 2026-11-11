class Class:  # expect: unused class
    plain = 1  # expect: unused attribute
    annotated: int = 2  # expect: unused attribute
    declared: int  # expect: unused attribute
    first, second = 3, 4  # expect: unused attribute, unused attribute
