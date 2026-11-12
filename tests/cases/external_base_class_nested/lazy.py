def __getattr__(name):
    if name == "Outer":
        from vendored.outer import Outer

        return Outer
    raise AttributeError(name)
