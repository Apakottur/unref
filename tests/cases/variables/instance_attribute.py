class Class:  # expect: 7 [unused-class]
    def __init__(self):
        self.plain = 1  # expect: 14 [unused-attribute] transitive
        self.annotated: int = 2  # expect: 14 [unused-attribute] transitive
        self.first, self.second = 3, 4  # expect: 14 [unused-attribute] transitive, 26 [unused-attribute] transitive

    def method(self):  # expect: 9 [unused-method] transitive
        # An attribute is reported once, where it is first assigned.
        self.plain = 5
        self.late = 6  # expect: 14 [unused-attribute] transitive

    @classmethod
    def class_method(cls):  # expect: 9 [unused-method] transitive
        cls.on_class = 7  # expect: 13 [unused-attribute] transitive

    def renamed_self(this):  # expect: 9 [unused-method] transitive
        this.renamed = 8  # expect: 14 [unused-attribute] transitive

    def nested(self):  # expect: 9 [unused-method] transitive
        # Not tracked: the first parameter of a nested function is not the instance.
        def inner(other):  # expect: 13 [unused-function] transitive
            other.inner = 9
            self.from_inner = 10
