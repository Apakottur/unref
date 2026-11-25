import importlib

import mod
from top.inner.mod import by_dotted_name

# A directory whose name is not an identifier is not a part of a dotted name.
from not_a_package.inner.mod import imported


def load(name):
    module = importlib.import_module(f"top.inner.{name}")
    return module.by_pattern()


mod.by_name(), by_dotted_name, load, imported
