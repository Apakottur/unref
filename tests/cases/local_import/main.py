# A name that is imported in a function is local to it.
operating_system = 1  # expect: 1 [unused-variable]
separator = 2  # expect: 1 [unused-variable]
os = 3  # expect: 1 [unused-variable]
join = 4  # expect: 1 [unused-variable]
# `import os.path` binds `os`, not `path`.
path = 5


def function():
    import os as operating_system
    import os.path
    from os.path import join
    from os.path import sep as separator

    return path, operating_system, join, separator, os


function()
