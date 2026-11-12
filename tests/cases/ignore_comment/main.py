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


@decorator  # unref: ignore
def comment_on_decorator():  # expect: unused function
    return 8


# unref: ignore
on_next_line = 9  # expect: unused variable

# The comment has to be a comment, and spelled this way.
in_string = "# unref: ignore"  # expect: unused variable
other_tool = 10  # other: ignore  # expect: unused variable
other_word = 11  # unref: ignored  # expect: unused variable
