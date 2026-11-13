from bases import Base, Parent


class TestFromBase(Base):
    pass


# Through a class that is not a test class.
class TestFromParent(Parent):
    pass


# A class that is not a test class does not run the tests that it inherits.
class Other(Base):  # expect: 7 [unused-class]
    pass
