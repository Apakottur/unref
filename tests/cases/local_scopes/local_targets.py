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
        index

    with open(__file__) as handle:
        handle

    try:
        items
    except ValueError as error:
        error

    if matched := len(items):
        matched

    first, (second, *_) = items
    first, second

    deleted = 8
    del deleted
    deleted


function([])
