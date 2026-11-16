# A `conf.py` that the option does not name is not that of Sphinx, even with its marker settings.
project = "Other"  # expect: 1 [unused-variable]
extensions = []  # expect: 1 [unused-variable]
