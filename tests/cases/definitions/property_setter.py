# A property is used by its setter, which reads it in the class body.
class Box:
    def __init__(self):
        self._size = 0

    @property
    def size(self):
        return self._size

    @size.setter
    def size(self, value):  # expect: 9 [unused-method]
        self._size = value


Box()
