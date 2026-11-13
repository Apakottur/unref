from sqlalchemy import Column, Integer
from sqlalchemy.ext.declarative import as_declarative
from sqlalchemy.orm import declarative_base

# A base class that a function of SQLAlchemy makes.
Base = declarative_base()


class Account(Base):
    __tablename__ = "accounts"

    id = Column(Integer, primary_key=True)
    plain = 1  # expect: 5 [unused-attribute]


# A base class that a decorator of SQLAlchemy makes, which is a model as well.
@as_declarative()
class DecoratedBase:
    id = Column(Integer, primary_key=True)


class Order(DecoratedBase):
    total = Column(Integer)


print(Account, Order)
