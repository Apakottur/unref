import sys

if sys.platform == "linux":
    PLATFORM = "linux"  # expect: 5 [unused-variable]
else:
    PLATFORM = "other"

try:
    import json
except ImportError:
    HAS_JSON = False  # expect: 5 [unused-variable]
else:
    HAS_JSON = True

with open(__file__) as file:
    CONTENT = file.read()  # expect: 5 [unused-variable]
