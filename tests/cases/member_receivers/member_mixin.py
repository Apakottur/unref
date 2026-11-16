# A mixin can call a method of another base class of a class that inherits from it.
class SaveMixin:
    def save(self):
        return self.serialize()


class Record:
    def serialize(self):
        return "record"


class Document(SaveMixin, Record):
    pass


class Image:
    def serialize(self):  # expect: 9 [unused-method]
        return "image"


Document().save()
Image()
