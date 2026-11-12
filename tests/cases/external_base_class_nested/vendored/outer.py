import logging


class Outer:
    class Base(logging.Handler):
        def custom(self):
            return self
