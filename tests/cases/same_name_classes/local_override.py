# An override with base classes covers the class that inherits from one, not another of its name.
from http.server import BaseHTTPRequestHandler


def first():
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            pass

    return Handler


def second():
    class Handler:
        def do_POST(self):  # expect: 13 [unused-method]
            pass

    return Handler


first(), second()
