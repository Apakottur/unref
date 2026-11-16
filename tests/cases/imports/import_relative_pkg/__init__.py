from .helpers import in_package

SHARED = 1
OTHER = 2  # expect: 1 [unused-variable]

print(in_package())
