class Class:  # expect: 7 [unused-class]
    def method(self):  # expect: 9 [unused-method] transitive
        return 1

    async def async_method(self):  # expect: 15 [unused-method] transitive
        return 2
