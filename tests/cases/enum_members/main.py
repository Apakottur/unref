import enums
from enums import Direction
from fruits import Berry, Fruit
import vegetables


def main():
    # Through an import, and through a package that gives it out.
    list(Direction), list(Fruit), Berry.STRAWBERRY, enums.Axis.X.name
    # Through a package that star imports it.
    list(vegetables.Root)


def later():
    # An import in a function.
    from enums import Weekday

    return Weekday


main()
later()
