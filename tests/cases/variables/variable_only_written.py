class Counter:
    total = 0

    def __init__(self):
        self.count = 0
        self.write_only = 0  # expect: 14 [unused-attribute]

    def increment(self):
        # Augmented assignment reads the attribute before updating it.
        self.count += 1
        self.write_only = 2


COUNT = 0  # expect: 1 [unused-variable]
COUNT += 1
Counter.total += 1
Counter().increment()
