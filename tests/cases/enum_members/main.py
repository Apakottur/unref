import enums
from enums import Direction
from fruits import Berry, Fruit


def main():
    # Through an import, and through a package that gives it out.
    print(list(Direction), list(Fruit), Berry.STRAWBERRY, enums.Axis.X.name)


def later():
    # An import in a function.
    from enums import Weekday

    return Weekday


main()
later()
