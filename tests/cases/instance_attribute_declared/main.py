class Class:  # expect: 7 [unused-class]
    declared: int  # expect: 5 [unused-attribute] transitive

    def __init__(self):
        # Already reported on the class.
        self.declared = 1


class Other:  # expect: 7 [unused-class]
    # The same name on another class is another attribute.
    declared: int  # expect: 5 [unused-attribute] transitive
