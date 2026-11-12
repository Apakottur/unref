from dataclasses import dataclass


class Base:
    pass


@dataclass
class Both(Base):
    field: int


@dataclass
class OnlyDecorator:
    field: int  # expect: unused attribute


class OnlyBase(Base):
    field: int  # expect: unused attribute


print(Both, OnlyDecorator, OnlyBase)
