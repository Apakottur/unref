class Class:  # expect: unused class
    @staticmethod
    def static_method():  # expect: unused method
        local = 1
        return local

    def keyword_only(*, value):  # expect: unused method
        value.attribute = 2

    def positional_only(self, /):  # expect: unused method
        self.attribute = 3  # expect: unused attribute
