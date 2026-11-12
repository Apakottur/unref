from helpers import used


# A star import in an unused function uses nothing.
def unused():  # expect: unused function
    from star import *  # noqa: F403

    return 1


print(used())
