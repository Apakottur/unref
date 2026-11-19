from abc import ABC, abstractmethod

from abstract_storage import Storage


def abstract_factory():
    class Base(ABC):
        @abstractmethod
        def save_local(self): ...

        print(save_local)

    return Base


def concrete_factory():
    class Base:
        def save_local(self):  # expect: 13 [unused-method]
            return b"base"

    class Fallback(Base):
        def save_local(self):  # expect: 13 [unused-method]
            return b"fallback"

    return Fallback


class Disk(Storage):
    @staticmethod
    def write_payload():
        return b"disk"


def fallback_factory():
    class Parent(Storage):
        pass

    class Fallback(Parent):
        @staticmethod
        def write_payload():
            return b"fallback"

    return Fallback


def unrelated_factory():
    class Parent:
        pass

    return Parent


print(abstract_factory(), concrete_factory(), fallback_factory(), unrelated_factory())
print(Disk.write_payload())
