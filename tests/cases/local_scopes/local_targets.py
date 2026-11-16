# Every way of binding a name in a function makes it local.
index = 1  # expect: 1 [unused-variable]
handle = 2  # expect: 1 [unused-variable]
error = 3  # expect: 1 [unused-variable]
matched = 4  # expect: 1 [unused-variable]
first = 5  # expect: 1 [unused-variable]
second = 6  # expect: 1 [unused-variable]
deleted = 7  # expect: 1 [unused-variable]


def function(items):
    for index in items:
        print(index)

    with open(__file__) as handle:
        print(handle)

    try:
        print(items)
    except ValueError as error:
        print(error)

    if matched := len(items):
        print(matched)

    first, (second, *_) = items
    print(first, second)

    deleted = 8
    del deleted
    print(deleted)


function([])
