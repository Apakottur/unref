import sys

if sys.platform == "linux":
    PLATFORM = "linux"  # expect: unused variable
else:
    PLATFORM = "other"

try:
    import json
except ImportError:
    HAS_JSON = False  # expect: unused variable
else:
    HAS_JSON = True

with open(__file__) as file:
    CONTENT = file.read()  # expect: unused variable
