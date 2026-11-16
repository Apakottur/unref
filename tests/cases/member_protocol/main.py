from typing import Protocol


# A class has the members of a protocol without inheriting from it.
class Sized(Protocol):
    def size(self) -> int: ...

    def weight(self) -> int: ...  # expect: 9 [unused-method]


class Box:
    def size(self) -> int:
        return 1


def measure(item: Sized) -> None:
    print(item)


measure(Box())
print(Box().size())
