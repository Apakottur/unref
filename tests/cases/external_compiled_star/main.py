from pkg import Base, Compiled
from pkg.dotted import Base as DottedBase
from pkg.stable import Base as StableBase
from pkg.tagged import Base as TaggedBase


# A base class is found through the star imports of a package, past a star import of a compiled module
# of the same name as the package, which is not followed as it has no source.
class Handler(Base):
    def emit(self, record):
        return record

    def own_method(self):  # expect: 9 [unused-method]
        return self


# A base class from the compiled module is not followed.
class FromCompiled(Compiled):
    def emit(self, record):  # expect: 9 [unused-method]
        return record


# A compiled module is found before a source of the same name, also with a platform tag in its name,
# such as `tagged.cpython-315-x86_64-linux-gnu.so` or `stable.abi3.so`.
class FromTagged(TaggedBase):
    def emit(self, record):  # expect: 9 [unused-method]
        return record


class FromStable(StableBase):
    def emit(self, record):  # expect: 9 [unused-method]
        return record


# A tag has no dot, so `dotted.x.y.so` is not the compiled module `dotted`, and its source is followed.
class FromDotted(DottedBase):
    def emit(self, record):
        return record


print(Handler, FromCompiled, FromTagged, FromStable, FromDotted)
