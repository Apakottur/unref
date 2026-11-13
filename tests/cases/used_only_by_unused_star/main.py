from helpers import used


# A star import in an unused function uses nothing.
def unused():  # expect: 5 [unused-function]
    from star import *  # noqa: F403

    return 1


print(used())
