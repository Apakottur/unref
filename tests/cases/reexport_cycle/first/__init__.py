# Two packages that give out names of each other, and a name of the package itself.
from second import shared
from first import shared as again
from first.first import first
