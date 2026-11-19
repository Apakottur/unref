# The variable of a comprehension that unpacks is local to it.
items = 1  # expect: 1 [unused-variable]
mapping = 2  # expect: 1 [unused-variable]
GROUPS = [[3], [4]]
MAPPINGS = [{"a": 5}]

[*items for items in GROUPS]
{*items for items in GROUPS}
{**mapping for mapping in MAPPINGS}
list(*items for items in GROUPS)
