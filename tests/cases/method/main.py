class Class:  # expect: unused class
    def method(self):  # expect: unused method
        return 1

    async def async_method(self):  # expect: unused method
        return 2
