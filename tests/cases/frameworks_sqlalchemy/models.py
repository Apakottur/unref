from typing import ClassVar

import sqlalchemy as sa
from sqlalchemy import Column, ForeignKey, Integer, String, event
from sqlalchemy.orm import (
    DeclarativeBase,
    Mapped,
    column_property,
    declared_attr,
    mapped_column,
    reconstructor,
    relationship,
    validates,
)
from sqlalchemy.orm.decl_api import DeclarativeBaseNoMeta, MappedAsDataclass


class Base(DeclarativeBase):
    pass


# The columns of a mixin are mapped in the models that inherit from it.
class TimestampMixin:
    created: Mapped[int] = mapped_column(Integer)
    note = "not mapped"  # expect: 5 [unused-attribute]

    @declared_attr
    def owner_id(cls):
        return mapped_column(ForeignKey("users.id"))


# SQLAlchemy maps the attributes of a model.
class User(TimestampMixin, Base):
    __tablename__ = "users"

    id: Mapped[int] = mapped_column(primary_key=True)
    name: Mapped[str]
    quoted: "Mapped[str]"
    email = Column(String)
    legacy = sa.Column(sa.String)
    upper = column_property(sa.func.upper(email))
    addresses: Mapped[list["Address"]] = relationship(back_populates="user")

    # Not mapped.
    limit: ClassVar[int] = 10  # expect: 5 [unused-attribute]
    label = "user"  # expect: 5 [unused-attribute]

    # SQLAlchemy calls the methods that its decorators register.
    @validates("email")
    def check_email(self, key, value):
        return value

    @reconstructor
    def on_load(self):
        pass

    # Other methods are still reported.
    def helper(self):  # expect: 9 [unused-method]
        return self


class Address(Base):
    __tablename__ = "addresses"

    id: Mapped[int] = mapped_column(primary_key=True)
    user: Mapped[User] = relationship(back_populates="addresses")


# Through the other base classes of SQLAlchemy.
class NoMetaBase(DeclarativeBaseNoMeta):
    pass


class Tag(NoMetaBase):
    id: Mapped[int] = mapped_column(primary_key=True)


class DataclassBase(MappedAsDataclass, DeclarativeBase):
    pass


class Item(DataclassBase):
    id: Mapped[int] = mapped_column(primary_key=True, init=False)


# SQLAlchemy calls an event listener, wherever it is.
@event.listens_for(User, "before_insert")
def before_insert(mapper, connection, target):
    pass


@sa.event.listens_for(User, "after_insert")
def after_insert(mapper, connection, target):
    pass


# Not a model.
class Plain:
    id: Mapped[int] = mapped_column(primary_key=True)  # expect: 5 [unused-attribute]
    other = Column(Integer)  # expect: 5 [unused-attribute]


print(User, Address, Tag, Item, Plain)
