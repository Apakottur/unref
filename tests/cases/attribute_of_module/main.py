import helpers
import helpers as renamed
import missing
import pkg
import pkg.mod
import pkg.other as renamed_submodule
from pkg import third

# More dots than there are packages: the import is not resolved.
from .. import beyond


class Object:
    def by_object(self):
        return 1


def function():
    import local

    return local.in_function()


# A parameter hides the imported module.
def hidden_by_parameter(helpers):
    return helpers.hidden_by_parameter()


print(
    # An attribute of an imported module refers to a name in that module.
    helpers.by_module(),
    renamed.by_renamed_module(),
    pkg.in_package(),
    pkg.mod.in_submodule(),
    renamed_submodule.in_renamed_submodule(),
    third.in_imported_submodule(),
    function(),
    # What follows the name is read from an object, and is matched by name.
    helpers.Holder.on_class,
    helpers.Holder().method(),
    # An attribute of a module that is not in the project is matched by name as well.
    missing.external,
    beyond.too_far,
    # An attribute of anything else does not refer to a name in a module.
    Object().by_object(),
    hidden_by_parameter(1),
    # A variable of a comprehension hides the imported module.
    [helpers.hidden_by_comprehension for helpers in []],
)
