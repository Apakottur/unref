# `getattr(value, "name")` and `hasattr(value, "name")` read `value.name`, as it is written.
import helpers


class Job:
    def by_self(self):
        return 1

    def by_class(self):
        return 2

    def by_chain(self):
        return 3

    def by_call_result(self):
        return 4

    def unused(self):  # expect: 9 [unused-method]
        return 5

    def run(self):
        return getattr(self, "by_self")()


class Other:
    def by_unknown(self):
        return 6

    def by_unknown_set(self):  # expect: 9 [unused-method]
        return 7


def read(value, name):
    setattr(value, "by_unknown_set", None)
    delattr(value, "by_unknown_set")
    return (
        getattr(value, "by_unknown")(),
        getattr(value, name),
    )


print(
    getattr(helpers, "by_getattr")(),
    hasattr(helpers, "by_hasattr"),
    getattr(helpers, "missing", helpers.by_default)(),
    setattr(helpers, "by_set", None),
    getattr(helpers, "by_name".upper()),
    getattr(Job, "by_class"),
    getattr(Job(), "by_call_result")(),
    getattr(Job, "run").__name__,
    getattr(Job(), "by_chain").__name__,
    read(Other(), "by_name"),
    Job().run(),
)
