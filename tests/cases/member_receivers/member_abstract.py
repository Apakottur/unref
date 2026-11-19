from abstract_storage import Storage


class DiskStorage(Storage):
    @staticmethod
    def write_payload():
        return b"disk"

    @staticmethod
    def reset_storage():
        return None


class FallbackStorage(Storage):
    @staticmethod
    def write_payload():
        return fallback_payload()

    @classmethod
    def read_payload(cls):  # expect: 9 [unused-method]
        return b"fallback"

    @staticmethod
    def reset_storage():  # expect: 9 [unused-method]
        return None


class ArchiveStorage(FallbackStorage):
    @staticmethod
    def write_payload():
        return b"archive"


class UnusedStorage(Storage):  # expect: 7 [unused-class]
    @staticmethod
    def write_payload():  # expect: 9 [unused-method] transitive
        return b"unused"


class UnrelatedStorage:
    @staticmethod
    def write_payload():  # expect: 9 [unused-method]
        return b"unrelated"


def fallback_payload():
    print(ArchiveStorage)
    return b"fallback"


print(DiskStorage.write_payload(), DiskStorage.reset_storage())
print(FallbackStorage, UnrelatedStorage)
