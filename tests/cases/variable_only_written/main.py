class Class:
    def __init__(self):
        self.count = 0  # expect: unused attribute

    def increment(self):
        # Writing is not a reference.
        self.count += 1
        self.count = 2


COUNT = 0  # expect: unused variable
COUNT += 1
Class().increment()
