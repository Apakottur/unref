import importlib

import plugins
from tools import formatting, parsing
from widgets import button


def load_plugin(name):
    # Some module of `plugins`, so `run` and `setup` are read in each module of `plugins`.
    module = importlib.import_module(f"plugins.{name}")
    module.setup()
    return module.run()


def load_backend():
    # Exactly `backends.memory`, as for `import backends.memory as backend`.
    backend = importlib.import_module("backends.memory")
    return backend.connect()


def load_handler(app, name):
    # A module named `*.handlers.*`, as Django names its management commands.
    module = importlib.import_module("apps.%s.handlers.%s" % (app, name))
    return module.handle()


def load_extension(path):
    # Any module, by a name only known when the code runs.
    module = importlib.import_module(path)
    return getattr(module, "register")()


def load_report(name):
    # Some module of `reports`, passed on, so a read of an attribute of any object can stand for the
    # names of each module of `reports`.
    module = importlib.import_module(f"reports.{name}")
    return module  # noqa: RET504


def load_any(path):
    # Any module, passed on: that does not pass on every module of the project.
    module = importlib.import_module(path)
    return module  # noqa: RET504


def load_legacy():
    module = __import__("backends.legacy", fromlist=["start"])
    return module.start()


def pick(mod):
    return mod.CONSTANT


def describe(mod):
    return mod.describe()


# `formatting` is passed on, so a read of an attribute of any object can stand for its names.
load_plugin, load_backend, load_handler, load_extension, load_any, load_legacy
load_report("daily").summary()
pick(formatting)
plugins.load, plugins.load_own
# Changing a module, or checking it, does not pass it on. `hasattr` reads the name, as `getattr` does.
setattr(parsing, "CONSTANT", 2)
isinstance(parsing, object), hasattr(parsing, "other")
# A package that binds the name of its module to a function: the function is passed, not the module.
describe(button)
