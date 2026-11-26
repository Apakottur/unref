# The config of an application reads the uppercase attributes of the classes that `from_object` is
# given, and of their base classes.
class Base:
    TESTING = False
    helper = None  # expect: 5 [unused-attribute]


class Production(Base):
    DATABASE_URI = "postgres://"


class Development(Base):
    DATABASE_URI = "sqlite://"


# `from_object` of something that is not an application.
class Unread:
    DEBUG = True  # expect: 5 [unused-attribute]
