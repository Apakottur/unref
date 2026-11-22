import importlib


def load(name):
    # Relative to this package.
    module = importlib.import_module(f".{name}", __package__)
    return module.teardown()


def load_own(name):
    module = importlib.import_module(f"{__name__}.{name}")
    return module.reset()
