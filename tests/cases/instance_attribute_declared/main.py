class Class:  # expect: unused class
    declared: int  # expect: unused attribute

    def __init__(self):
        # Already reported on the class.
        self.declared = 1


class Other:  # expect: unused class
    # The same name on another class is another attribute.
    declared: int  # expect: unused attribute
