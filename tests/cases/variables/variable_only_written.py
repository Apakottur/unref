class Class:
    def __init__(self):
        self.count = 0  # expect: 14 [unused-attribute]

    def increment(self):
        # Writing is not a reference.
        self.count += 1
        self.count = 2


COUNT = 0
COUNT += 1
Class().increment()

WRITE_ONLY = 0  # expect: 1 [unused-variable]
WRITE_ONLY = 1

NAMESPACE_COUNT = 0
globals()["NAMESPACE_COUNT"] += 1

GLOBAL_COUNT = 0


def increment():
    global GLOBAL_COUNT
    GLOBAL_COUNT += 1


increment()
