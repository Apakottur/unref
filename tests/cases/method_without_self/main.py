class Class:  # expect: 7 [unused-class]
    @staticmethod
    def static_method():  # expect: 9 [unused-method]
        local = 1
        return local

    def keyword_only(*, value):  # expect: 9 [unused-method]
        value.attribute = 2

    def positional_only(self, /):  # expect: 9 [unused-method]
        self.attribute = 3  # expect: 14 [unused-attribute]
