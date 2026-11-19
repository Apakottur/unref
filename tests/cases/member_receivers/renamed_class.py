from renamed_class_pkg.proxy import Delegated
from renamed_class_pkg.provider import LocalReplacement, ModuleReplacement
from renamed_class_pkg.wild import Starred
from renamed_class_pkg.worker import (
    API,
    DefaultValue,
    GlobalImport,
    GlobalValue,
    Looped,
    ModuleValue,
    Rebound,
    Walrus,
)


class Other:
    default_mode = -1  # expect: 5 [unused-attribute]

    def run_exported(self):  # expect: 9 [unused-method]
        return -1


Other()
print(API().run_exported(), API.default_mode)
# Conflicting bindings use the ordinary name-based lookup.
print(Delegated().shadowed_action(), Rebound().rebound_action())
print(Looped().loop_action(), Starred().star_action())
print(GlobalValue().global_action(), GlobalImport().imported_action())
print(Walrus().walrus_action())
print(DefaultValue().default_action())

ModuleValue = ModuleReplacement
print(ModuleValue().module_action())


def consume():
    from renamed_class_pkg.worker import LocalValue

    LocalValue = LocalReplacement
    return LocalValue().local_action()


print(consume())


def consume_nonlocal():
    from renamed_class_pkg.worker import LocalValue

    def setup():
        def replace():
            nonlocal LocalValue
            LocalValue = LocalReplacement

        replace()

    setup()
    return LocalValue().nonlocal_action()


def consume_parameter(LocalValue, use_default):
    if use_default:
        from renamed_class_pkg.worker import LocalValue

    return LocalValue().parameter_action()


print(consume_nonlocal(), consume_parameter(LocalReplacement, False))
