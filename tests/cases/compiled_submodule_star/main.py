import pkg


# A base class from a compiled submodule is not followed, nor are the star imports of the package
# followed again for it, through the same submodule or the other one.
class Handler(pkg.fast.Base):
    def emit(self, record):  # expect: 9 [unused-method]
        return record


class Other(pkg.slow.Base):
    def emit(self, record):  # expect: 9 [unused-method]
        return record


print(Handler, Other)
