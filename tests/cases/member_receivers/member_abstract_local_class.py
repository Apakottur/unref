from abstract_storage import Storage


class DiskStorage(Storage):
    @staticmethod
    def write_payload():
        return b"disk"


def unused_factory_first():  # expect: 5 [unused-function]
    class Fallback(Storage):  # expect: 11 [unused-class] transitive
        @staticmethod
        def write_payload():  # expect: 13 [unused-method] transitive
            return b"unused"

    return Fallback


def used_factory_second():
    class Fallback(Storage):
        @staticmethod
        def write_payload():
            return b"fallback"

    return Fallback


def used_factory_first():
    class Archive(Storage):
        @staticmethod
        def write_payload():
            return b"archive"

    return Archive


def unused_factory_second():  # expect: 5 [unused-function]
    class Archive(Storage):  # expect: 11 [unused-class] transitive
        @staticmethod
        def write_payload():  # expect: 13 [unused-method] transitive
            return b"unused"

    return Archive


print(DiskStorage.write_payload(), used_factory_second(), used_factory_first())
