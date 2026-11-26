# The config of an application reads the uppercase variables of a module that `from_object` names.
SECRET_KEY = "secret"
cache_dir = "cache"  # expect: 1 [unused-variable]
