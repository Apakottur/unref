from .provider import Service as API
from .provider import ShadowReplacement, ShadowSource as Adjusted
from .provider import ReboundSource as Rebound
from .provider import ReboundReplacement as Rebound
from .provider import LoopReplacement, LoopSource as Looped
from .provider import DefaultReplacement, DefaultSource as DefaultValue
from .provider import GlobalImportSource as GlobalImport, GlobalReplacement
from .provider import LocalSource as LocalValue
from .provider import ModuleSource as ModuleValue
from .provider import WalrusReplacement, WalrusSource as Walrus

Adjusted = ShadowReplacement

for Looped in [LoopReplacement]:
    pass


def replace_global_value():
    global GlobalValue
    GlobalValue = GlobalReplacement


def replace_global_import():
    global GlobalImport
    from .provider import GlobalImportReplacement as GlobalImport


def default_value(default=(DefaultValue := DefaultReplacement)):
    return default


# The function is defined before the initial import but runs after it.
from .provider import GlobalSource as GlobalValue

replace_global_value()
replace_global_import()
[(Walrus := WalrusReplacement) for _ in (None,)]
default_value()
