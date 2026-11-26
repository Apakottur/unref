def header_property(name):
    return property(lambda self: name)


# A decorator that is an attribute of a name bound earlier in the class body registers the method on
# that object.
class Response:
    etag = header_property("ETag")

    @etag.register_setter
    def _set_etag(self, value):
        return value

    @etag.validator()
    def check_etag(self, value):
        return value

    @later.register
    def before(self):  # expect: 9 [unused-method]
        return 2

    later = header_property("Later")  # expect: 5 [unused-attribute]

    @header_property("Other").register
    def not_a_name(self):  # expect: 9 [unused-method]
        return 3


Response()
