# Python calls dunder methods implicitly.
class Class:  # expect: unused class
    def __init__(self):
        self.value = 1  # expect: unused attribute

    def __str__(self):
        return "text"

    async def __aenter__(self):
        return self
