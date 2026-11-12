import concurrent.futures
from concurrent.futures import ProcessPoolExecutor

import lazy
import not_lazy


# A base class that a module gives out from its `__getattr__` is followed through the import in the
# function: `concurrent.futures` imports `ThreadPoolExecutor` from `concurrent.futures.thread` there.
class Executor(concurrent.futures.ThreadPoolExecutor):
    def submit(self, fn, /, *args, **kwargs):
        return super().submit(fn, *args, **kwargs)

    def own_method(self):  # expect: unused method
        return self


class Processes(ProcessPoolExecutor):
    def submit(self, fn, /, *args, **kwargs):
        return super().submit(fn, *args, **kwargs)


# A module of the project is followed the same way.
class FromLazy(lazy.Handler):
    def emit(self, record):
        return record


# A name that `__getattr__` gives out without importing it is not followed.
class FromComputed(lazy.Encoder):
    def default(self, o):  # expect: unused method
        return o


# An import that leads nowhere is not followed.
class FromTooFar(lazy.TooFar):
    def emit(self, record):  # expect: unused method
        return record


# An import in another function is not given out.
class FromOtherFunction(not_lazy.Handler):
    def emit(self, record):  # expect: unused method
        return record


print(Executor, Processes, FromLazy, FromComputed, FromTooFar, FromOtherFunction)
