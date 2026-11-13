from sqlalchemy import Column, Integer

from legacy import Account


# A model of a model of another module, whose base class is known only there.
class Premium(Account):
    level = Column(Integer)
    plain = 1  # expect: 5 [unused-attribute]


print(Premium)
