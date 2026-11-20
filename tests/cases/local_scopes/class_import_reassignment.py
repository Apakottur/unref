class Runner:
    from math import sqrt as alias

    print(alias(4))
    alias = 3
    print(alias)
    unused = 5  # expect: 5 [unused-attribute]


class Result:
    from math import sqrt as value

    value = value(9)  # expect: 5 [unused-attribute]


print(Runner, Result)
