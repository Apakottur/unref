# The column of a finding is that of the name of the definition, as ruff and ty point to it, and counts
# characters rather than bytes.


def  spaced():  # expect: 6 [unused-function]
    pass


async	def	tabbed():  # expect: 11 [unused-function]
    pass


@staticmethod
def decorated():  # expect: 5 [unused-function]
    pass


# The name is on the line after a line continuation.
def \
    continued():  # expect: 5 [unused-function]
    pass


class \
    ContinuedClass:  # expect: 5 [unused-class]
    def method(self):  # expect: 9 [unused-method]
        self . spaced_attribute = 1  # expect: 16 [unused-attribute]
        (self.
         split_attribute) = 2  # expect: 10 [unused-attribute]


é = 1; ü = 2  # expect: 1 [unused-variable], 8 [unused-variable]
