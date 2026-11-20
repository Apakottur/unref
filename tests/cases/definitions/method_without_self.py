class Class:  # expect: 7 [unused-class]
    @staticmethod
    def static_method():  # expect: 9 [unused-method] transitive
        return 1

    def keyword_only(*, value):  # expect: 9 [unused-method] transitive
        value.attribute = 2

    def positional_only(self, /):  # expect: 9 [unused-method] transitive
        self.attribute = 3  # expect: 14 [unused-attribute] transitive
