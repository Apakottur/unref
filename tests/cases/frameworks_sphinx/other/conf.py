# A `conf.py` that sets none of `project`, `extensions` and `html_theme` is not that of Sphinx.
database_url = "sqlite://"  # expect: 1 [unused-variable]


def setup(app):  # expect: 5 [unused-function]
    pass
