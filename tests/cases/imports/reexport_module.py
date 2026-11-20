from reexport_module_pkg.worker import source

source.value()


class Other:
    def value(self):  # expect: 9 [unused-method]
        return 0


Other()
