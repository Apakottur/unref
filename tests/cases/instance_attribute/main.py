class Class:  # expect: unused class
    def __init__(self):  # expect: unused method
        self.plain = 1  # expect: unused attribute
        self.annotated: int = 2  # expect: unused attribute
        self.first, self.second = 3, 4  # expect: unused attribute, unused attribute

    def method(self):  # expect: unused method
        # An attribute is reported once, where it is first assigned.
        self.plain = 5
        self.late = 6  # expect: unused attribute

    @classmethod
    def class_method(cls):  # expect: unused method
        cls.on_class = 7  # expect: unused attribute

    def renamed_self(this):  # expect: unused method
        this.renamed = 8  # expect: unused attribute

    def nested(self):  # expect: unused method
        # Not tracked: the first parameter of a nested function is not the instance.
        def inner(other):  # expect: unused function
            other.inner = 9
            self.from_inner = 10
