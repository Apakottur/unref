from abc import ABC, abstractmethod as rebound


def abstract_factory():
    class AbstractStorage(ABC):
        @staticmethod
        @required
        def save_manifest(): ...

    return AbstractStorage


def concrete_factory():
    class ConcreteStorage(ABC):
        @staticmethod
        @rebound
        def save_config():
            return b"base"

    return ConcreteStorage


def identity(method):
    return method


from abc import abstractmethod as required

rebound = identity
AbstractStorage = abstract_factory()
ConcreteStorage = concrete_factory()
# The abstract factory already ran with abc.abstractmethod.
required = identity


class Disk(AbstractStorage):
    @staticmethod
    def save_manifest():
        return b"disk"


class Fallback(AbstractStorage):
    @staticmethod
    def save_manifest():
        return b"fallback"


class Cache(ConcreteStorage):
    @staticmethod
    def save_config():
        return b"cache"


class CacheFallback(ConcreteStorage):
    @staticmethod
    def save_config():  # expect: 9 [unused-method]
        return b"fallback"


print(Disk.save_manifest(), Fallback(), Cache.save_config(), CacheFallback())
