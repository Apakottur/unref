from abc import ABC, abstractmethod


class AbstractStorage(ABC):
    from abc import abstractmethod as required

    @staticmethod
    @required
    def write_record(): ...


class Disk(AbstractStorage):
    @staticmethod
    def write_record():
        return b"disk"


class Fallback(AbstractStorage):
    @staticmethod
    def write_record():
        return b"fallback"


def identity(method):
    return method


class ConcreteStorage(ABC):
    abstractmethod = identity

    @staticmethod
    @abstractmethod
    def write_cache():
        return b"base"


class DiskCache(ConcreteStorage):
    @staticmethod
    def write_cache():
        return b"disk"


class FallbackCache(ConcreteStorage):
    @staticmethod
    def write_cache():  # expect: 9 [unused-method]
        return b"fallback"


print(Disk.write_record(), Fallback, DiskCache.write_cache(), FallbackCache)
