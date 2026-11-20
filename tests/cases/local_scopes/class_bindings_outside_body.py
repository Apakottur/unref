class Init:
    global STATUS
    STATUS = 5
    UNUSED = 9  # expect: 5 [unused-attribute]


class Parent:
    pass


class Child((parent := Parent)):
    pass


print(Init, STATUS)
print(Child, parent is Parent, "parent" in vars(Child))


def check_scope():
    status = object()

    class Nested:
        global status
        status = 7
        print(status)

    print(status, Nested)


check_scope()
