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
