from renamed_class_pkg.proxy import Delegated
from renamed_class_pkg.wild import Starred
from renamed_class_pkg.worker import API, Looped, Rebound


class Other:
    default_mode = -1  # expect: 5 [unused-attribute]

    def run_exported(self):  # expect: 9 [unused-method]
        return -1


Other()
print(API().run_exported(), API.default_mode)
# Conflicting bindings use the ordinary name-based lookup.
print(Delegated().shadowed_action(), Rebound().rebound_action())
print(Looped().loop_action(), Starred().star_action())
