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

ENCLOSED_GLOBAL_COUNT = 0
ENCLOSED_NONLOCAL_COUNT = 0  # expect: 1 [unused-variable]


def increment_from_nested_function():
    ENCLOSED_GLOBAL_COUNT = 2
    ENCLOSED_NONLOCAL_COUNT = 2

    def increment():
        global ENCLOSED_GLOBAL_COUNT
        nonlocal ENCLOSED_NONLOCAL_COUNT
        ENCLOSED_GLOBAL_COUNT += 1
        ENCLOSED_NONLOCAL_COUNT += 1

    increment()


increment_from_nested_function()
