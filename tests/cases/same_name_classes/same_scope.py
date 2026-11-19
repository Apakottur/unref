# Classes of one name in one scope are one class, as either can be the one that is bound.
import logging
import sys

if sys.platform == "win32":

    class Handler(logging.Handler):
        def emit(self, record):
            pass

else:

    class Handler:
        def emit(self, record):
            pass


Handler()
