# A base class can call a method that a subclass overrides.
class Report:
    def render(self):
        return self.header() + self.body()

    def header(self):
        return "report"

    def body(self):
        raise NotImplementedError


class Sales(Report):
    def body(self):
        return "sales"


class Unrelated:
    def body(self):  # expect: 9 [unused-method]
        return "unrelated"


Sales().render()
Unrelated()
