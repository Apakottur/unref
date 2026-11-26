# A relative import in a directory without an `__init__.py` and without a package above it goes up
# through the directories of the name from the project root.
from .. import helpers
from . import forms
from .lib import View


class Index(View):
    pass


Index, forms.form(), helpers.helper()
