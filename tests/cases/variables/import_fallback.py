try:
    import json
except ImportError:
    json = None

try:
    import ssl
except ImportError:
    ssl = None

try:
    import csv
except ImportError:
    csv = None  # expect: 5 [unused-variable]


def dumps():
    return json.dumps


def context():
    return ssl.create_default_context()


dumps()
context()
