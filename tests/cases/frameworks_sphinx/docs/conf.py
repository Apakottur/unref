import os

# Sphinx reads the settings at the top level, its own and those of its extensions.
project = "Example"
extensions = ["sphinx.ext.intersphinx", "sphinx.ext.linkcode", "myst_parser"]
html_theme = "alabaster"
intersphinx_mapping = {"python": ("https://docs.python.org/3", None)}
myst_enable_extensions = ["colon_fence"]

if os.environ.get("READTHEDOCS"):
    html_baseurl = os.environ["READTHEDOCS_CANONICAL_URL"]

# No setting starts with an underscore.
_unused_helper = os.path.dirname(__file__)  # expect: 1 [unused-variable]


# Sphinx calls `setup`, and `sphinx.ext.linkcode` calls `linkcode_resolve`.
def setup(app):
    app.connect("source-read", on_source_read)


def on_source_read(app, name, source):
    return source


def linkcode_resolve(domain, info):
    return None


# The rest of the file can hold dead code.
def unused_function():  # expect: 5 [unused-function]
    pass


class UnusedClass:  # expect: 7 [unused-class]
    extensions = []  # expect: 5 [unused-attribute] transitive
