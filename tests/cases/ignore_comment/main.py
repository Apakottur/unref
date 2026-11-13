def function():  # unref: ignore
    return 1


class Class:  # unref: ignore
    on_class = 1  # unref: ignore

    def method(self):  # unref: ignore
        self.on_instance = 2  # unref: ignore


variable = 3  # unref: ignore

# The comment can share the line with other comments, and its spacing is free.
after_other_comment = 4  # type: ignore[assignment]  # unref: ignore
before_other_comment = 5  # unref: ignore  # A reason.
spacing = 6  #unref:ignore


# The comment is on the line of the `def`, under the decorators.
@decorator
def decorated():  # unref: ignore
    return 7


@decorator  # unref: ignore  # expect: 13 [unused-ignore-comment]
def comment_on_decorator():  # expect: 5 [unused-function]
    return 8


# A comment that ignores nothing is reported.
# unref: ignore  # expect: 1 [unused-ignore-comment]
on_next_line = 9  # expect: 1 [unused-variable]


def referenced():  # unref: ignore  # expect: 20 [unused-ignore-comment]
    return 10


class Referenced:
    def referenced_method(self):  # unref: ignore  # expect: 35 [unused-ignore-comment]
        return 11


print(referenced(), Referenced().referenced_method())  # unref: ignore  # expect: 56 [unused-ignore-comment]
visit_name = 12  # unref: ignore  # expect: 18 [unused-ignore-comment]
__version__ = 13  # unref: ignore  # expect: 19 [unused-ignore-comment]

# One comment covers every finding of its line.
first, second = 14, 15  # unref: ignore

# The comment has to be a comment, and spelled this way.
in_string = "# unref: ignore"  # expect: 1 [unused-variable]
other_tool = 17  # other: ignore  # expect: 1 [unused-variable]
other_word = 16  # unref: ignored  # expect: 1 [unused-variable]
