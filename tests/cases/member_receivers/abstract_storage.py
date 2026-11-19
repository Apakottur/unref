from abc import ABC, abstractmethod as required


class Storage(ABC):
    @staticmethod
    @required
    def write_payload(): ...

    @classmethod
    @required
    def read_payload(cls): ...  # expect: 9 [unused-method]

    @staticmethod
    def reset_storage():
        return None
