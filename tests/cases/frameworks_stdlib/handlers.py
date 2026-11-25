import importlib.abc
from http.server import BaseHTTPRequestHandler, SimpleHTTPRequestHandler


# `BaseHTTPRequestHandler` calls the method named `do_` and the method of the request, which is
# uppercase.
class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        return None

    def do_PATCH(self):
        return None

    def do_cleanup(self):  # expect: 9 [unused-method]
        return None


# So does a subclass of a subclass of it from the standard library.
class Files(SimpleHTTPRequestHandler):
    def do_PUT(self):
        return None


# The import system calls `find_spec` of a finder and `exec_module` of a loader.
class Finder(importlib.abc.MetaPathFinder):
    def find_spec(self, fullname, path, target=None):
        return None

    def describe(self):  # expect: 9 [unused-method]
        return None


class Loader(importlib.abc.Loader):
    def exec_module(self, module):
        return None


# Not a handler: a method named like one is reported.
class Plain:
    def do_POST(self):  # expect: 9 [unused-method]
        return None

    def find_spec(self, fullname, path, target=None):  # expect: 9 [unused-method]
        return None


Handler, Files, Finder, Loader, Plain
