# A member read from a class, or from a call of it, is matched in the class and its base classes. A
# subclass is left out: `Shape.describe(square)` calls the method of `Shape`.
import shapes
from shapes import models
from shapes.models import Circle, Shape

shapes.Square.sides
models.Square().area()
Shape.describe(models.Square())
Circle
