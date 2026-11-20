# An implementation of a used abstract method is used when its class is, as it is what makes the class
# implement the abstract method. An abstract method that only unused code reaches makes its
# implementations transitive.
import abc
from abc import ABC, abstractmethod


class Storage(ABC):
    @staticmethod
    @abstractmethod
    def save(data: bytes) -> None: ...

    @abc.abstractmethod
    def close(self) -> None: ...

    @abstractmethod
    def flush(self) -> None: ...  # expect: 9 [unused-method]

    def describe(self) -> str:
        return "storage"


class Disk(Storage):
    @staticmethod
    def save(data: bytes) -> None:
        data

    def close(self) -> None: ...

    def flush(self) -> None: ...  # expect: 9 [unused-method] transitive

    def describe(self) -> str:
        return "disk"


class Null(Storage):
    @staticmethod
    def save(data: bytes) -> None:
        raise RuntimeError

    def close(self) -> None: ...

    def flush(self) -> None: ...  # expect: 9 [unused-method] transitive

    def describe(self) -> str:  # expect: 9 [unused-method]
        return "null"


class BufferedNull(Null):
    def save(self, data: bytes) -> None: ...


class Unused(Storage):  # expect: 7 [unused-class]
    @staticmethod
    def save(data: bytes) -> None: ...  # expect: 9 [unused-method] transitive


class Task(ABC):
    @abstractmethod
    def run(self) -> None: ...  # expect: 9 [unused-method] transitive


class PrintTask(Task):
    def run(self) -> None:  # expect: 9 [unused-method] transitive
        "run"


def run_task(task: Task) -> None:  # expect: 5 [unused-function]
    Task.run(task)


Disk.save(b"data")
Storage.close(Disk())
Disk().describe(), Null, BufferedNull, PrintTask
