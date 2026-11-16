def used():
    from used_only_by_unused_star_star_used import *  # noqa: F403

    return 1
