def pair():
    return 1, 2


first, unused_second = pair()
third, also_ignored_value = pair()
first, third

# The default pattern no longer applies.
fourth, _fifth = pair()  # expect: 9 [unused-variable]
fourth

# `_` is never reported, whatever the pattern.
sixth, _ = pair()
sixth

# Where the binding can be dropped, the name is reported.
unused_plain = 3  # expect: 1 [unused-variable]
