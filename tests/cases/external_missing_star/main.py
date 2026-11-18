from pkg import Base, solver


# A star import of a module that is not installed gives out no names, so a name that is followed
# through it is not looked for in the package above it, which would star import it again.
def solve(matrix):
    values = solver.run(matrix)
    return values


# A base class is still found through the other star imports of the package.
class Handler(Base):
    def emit(self, record):
        return record

    def own_method(self):  # expect: 9 [unused-method]
        return self


print(solve, Handler)
