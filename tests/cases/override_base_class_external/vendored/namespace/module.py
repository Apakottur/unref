# A directory without an `__init__.py` is a namespace package, which has no source of its own.
from enum import Enum


class Base(Enum):
    pass
