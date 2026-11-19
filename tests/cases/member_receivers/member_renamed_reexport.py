# A class that a package gives out under another name is matched by its members, so a method of the
# same name in another class is still unused.
from services import API


class Other:
    def execute(self):  # expect: 9 [unused-method]
        return -1


Other()
API().execute()
