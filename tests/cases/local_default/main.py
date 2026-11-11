import functools

# A default value, an annotation and a decorator are not in the scope of the function.
limit = 1
Kind = int
wrapper = functools.cache


@wrapper
def function(limit=limit, kind: Kind = 2) -> Kind:
    wrapper = Kind = 3
    return limit + kind + wrapper + Kind


print(lambda limit=limit: limit)
function()
