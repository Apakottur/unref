import logging


# A member that is not in the class or its base classes of the project is matched by name, since it
# can come from a base class outside the project.
class Handler(logging.Handler):
    def emit(self, record):
        self.format(record)

    def own_method(self):  # expect: 9 [unused-method]
        return self


class Report:
    def format(self, record):
        return str(record)

    def own_method(self):  # expect: 9 [unused-method]
        return self


Handler()
Report()
