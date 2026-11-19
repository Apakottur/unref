class Service:
    default_mode = 7

    def run_exported(self):
        return 3


class ShadowSource:
    def shadowed_action(self):
        return 1


class ShadowReplacement:
    def shadowed_action(self):
        return 2


class ReboundSource:
    def rebound_action(self):
        return 4


class ReboundReplacement:
    def rebound_action(self):
        return 5


class LoopSource:
    def loop_action(self):
        return 6


class LoopReplacement:
    def loop_action(self):
        return 7


class StarSource:
    def star_action(self):
        return 8


class StarReplacement:
    def star_action(self):
        return 9


class GlobalSource:
    def global_action(self):
        return 10


class GlobalReplacement:
    def global_action(self):
        return 11


class GlobalImportSource:
    def imported_action(self):
        return 12


class GlobalImportReplacement:
    def imported_action(self):
        return 13


class WalrusSource:
    def walrus_action(self):
        return 14


class WalrusReplacement:
    def walrus_action(self):
        return 15


class DefaultSource:
    def default_action(self):
        return 16


class DefaultReplacement:
    def default_action(self):
        return 17


class ModuleSource:
    def module_action(self):
        return 18


class ModuleReplacement:
    def module_action(self):
        return 19


class LocalSource:
    def local_action(self):
        return 20

    def nonlocal_action(self):
        return 22

    def parameter_action(self):
        return 24


class LocalReplacement:
    def local_action(self):
        return 21

    def nonlocal_action(self):
        return 23

    def parameter_action(self):
        return 25


class RemoteSource:
    def remote_action(self):
        return 26


class RemoteReplacement:
    def remote_action(self):
        return 27
