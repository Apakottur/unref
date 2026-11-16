# A member read from an object whose class is not known is matched by name in every class.
class Engine:
    def start(self):
        return 1

    def halt(self):
        return 2


class Timer:
    def start(self):
        return 3

    @staticmethod
    def halt_all(engine):
        # The first parameter of a static method is not the instance.
        return engine.halt()

    def restart(self):
        return type(self).start(self)


def run(machine):
    return machine.start()


run(Engine())
Timer.halt_all(Engine())
Timer().restart()
