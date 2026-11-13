from sqlalchemy import Column, Integer


# Not a base class of models: a function with the name of SQLAlchemy's, outside SQLAlchemy.
def declarative_base():
    return object


Base = declarative_base()


class NotModel(Base):
    id = Column(Integer)  # expect: 5 [unused-attribute]


print(NotModel)
