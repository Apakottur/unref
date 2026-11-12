# A name that a pattern captures is local.
whole = 1  # expect: 1 [unused-variable]
rest = 2  # expect: 1 [unused-variable]
remaining = 3  # expect: 1 [unused-variable]
captured = 4  # expect: 1 [unused-variable]


def function(subject):
    match subject:
        case [captured, *rest]:
            return captured, rest
        case {"key": captured, **remaining}:
            return captured, remaining
        case _ as whole:
            return whole


function([])
