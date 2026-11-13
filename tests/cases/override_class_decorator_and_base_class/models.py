from dataclasses import dataclass


class Base:
    pass


@dataclass
class Both(Base):
    field: int


@dataclass
class OnlyDecorator:
    field: int  # expect: 5 [unused-attribute]


class OnlyBase(Base):
    field: int  # expect: 5 [unused-attribute]


print(Both, OnlyDecorator, OnlyBase)
