# A member read from an object whose class is not known is matched by name in every class.
class Engine:
    def start(self):
        return 1

    def stop(self):
        return 2


class Timer:
    def start(self):
        return 3

    @staticmethod
    def stop_all(engine):
        # The first parameter of a static method is not the instance.
        return engine.stop()

    def restart(self):
        return type(self).start(self)


def run(machine):
    return machine.start()


run(Engine())
Timer.stop_all(Engine())
Timer().restart()
