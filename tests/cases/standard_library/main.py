import handlers


# The standard library is the one of the newest version under the prefix of the Python, unless the
# `pyvenv.cfg` of the environment names a version.
class Handler(handlers.Base):
    def newest(self):
        return self

    def oldest(self):  # expect: 9 [unused-method]
        return self


Handler
