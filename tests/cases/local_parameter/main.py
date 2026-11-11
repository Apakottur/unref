# A read of a parameter is not a reference to something of the same name.
value = 1  # expect: unused variable
args = 2  # expect: unused variable
kwargs = 3  # expect: unused variable
only = 4  # expect: unused variable
keyword = 5  # expect: unused variable


def function(only, /, value, *args, keyword, **kwargs):
    return only, value, args, keyword, kwargs


function(1, 2, keyword=3)
