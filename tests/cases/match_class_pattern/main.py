from dataclasses import dataclass


@dataclass
class Point:
    x: int
    y: int
    label: str  # expect: unused attribute


@dataclass
class Line:
    start: Point
    end: Point


def where(point):
    # A keyword in a class pattern reads the attribute of the subject.
    match point:
        case Point(x=0, y=0):
            return "origin"
    return "else"


def length(line):
    # The keywords of a class pattern nested in another one are read as well.
    match line:
        case Line(start=Point(x=0), end=Point(x=x)):
            return x
    return 0


where(Point(1, 2, ""))
length(Line(Point(0, 0, ""), Point(1, 1, "")))
