from used_only_by_unused_star_helpers import used


# A star import in an unused function uses nothing.
def unused():  # expect: 5 [unused-function]
    from used_only_by_unused_star_star import *

    return 1


used()
