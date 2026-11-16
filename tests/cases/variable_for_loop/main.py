# The target of a `for` loop is not collected: like Ruff, which leaves an unused loop variable to a
# rule of its own, unref does not report it as an unused variable.
for item in range(3):
    pass

for index, value in enumerate([4]):
    pass


class Holder:
    for member in range(3):
        pass


print(Holder)
