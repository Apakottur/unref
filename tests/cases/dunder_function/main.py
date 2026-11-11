# A module-level `__getattr__` is called for the missing attributes of the module.
def __getattr__(name):
    raise AttributeError(name)
