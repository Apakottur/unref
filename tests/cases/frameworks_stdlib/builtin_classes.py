import io


# A builtin class has no Python source to read its methods from.
class Reader(io.RawIOBase):
    def readable(self):
        return True

    def readinto(self, buffer):
        return 0

    def own_method(self):  # expect: 9 [unused-method]
        return self


class Mapping(dict):
    def keys(self):
        return []

    def setdefault(self, key, default=None):
        return default

    def own_method(self):  # expect: 9 [unused-method]
        return self


class Items(list):
    def append(self, item):
        return item


class Text(io.TextIOBase):
    def readable(self):
        return True

    def own_method(self):  # expect: 9 [unused-method]
        return self


Reader, Mapping, Items, Text
