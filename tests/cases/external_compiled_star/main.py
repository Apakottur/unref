from pkg import Base, Compiled


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


print(Handler, FromCompiled)
