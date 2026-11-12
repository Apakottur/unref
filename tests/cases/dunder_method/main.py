# Python calls dunder methods implicitly.
class Class:  # expect: 7 [unused-class]
    def __init__(self):
        self.value = 1  # expect: 14 [unused-attribute]

    def __str__(self):
        return "text"

    async def __aenter__(self):
        return self
