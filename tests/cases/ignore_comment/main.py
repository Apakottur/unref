def function():  # unref: ignore[unused-function]
    return 1


class Class:  # unref: ignore[unused-class]
    on_class = 1  # unref: ignore[unused-attribute]

    def method(self):  # unref: ignore[unused-method]
        self.on_instance = 2  # unref: ignore[unused-attribute]


variable = 3  # unref: ignore[unused-variable]

# The comment can share the line with other comments, and its spacing is free, as with ty.
after_other_comment = 4  # type: ignore[assignment]  # unref: ignore[unused-variable]
before_other_comment = 5  # unref: ignore[unused-variable]  # A reason.
reason_after_bracket = 6  # unref: ignore[unused-variable] A reason.
no_spaces = 7  #unref:ignore[unused-variable]
more_spaces = 8  #  unref:  ignore  [ unused-variable ,  ]


# The comment is on the line of the name, under the decorators.
@decorator
def decorated():  # unref: ignore[unused-function]
    return 9


@decorator  # unref: ignore[unused-function]  # expect: 13 [unused-ignore-comment]
def comment_on_decorator():  # expect: 5 [unused-function]
    return 10


# A comment ignores only the findings with one of its codes.
def other_code():  # unref: ignore[unused-class]  # expect: 5 [unused-function], 20 [unused-ignore-comment]
    return 11


# One comment, or several, can ignore several findings of its line, each by its code.
class First: first_attribute = 12  # unref: ignore[unused-class, unused-attribute]
class Second: second_attribute = 13  # unref: ignore[unused-class]  # unref: ignore[unused-attribute]
pair_first, pair_second = 14, 15  # unref: ignore[unused-variable]

# A code that ignores nothing is reported at the code, unless every code of the comment ignores nothing.
partly_used = 16  # unref: ignore[unused-variable, unused-function]  # expect: 52 [unused-ignore-comment]
both_unused = 17  # unref: ignore[unused-class, unused-function]  # expect: 1 [unused-variable], 19 [unused-ignore-comment]
# A code given twice ignores a finding once, and the first is reported as with ty.
twice = 18  # unref: ignore[unused-variable, unused-variable]  # expect: 29 [unused-ignore-comment]


# A comment that ignores nothing is reported.
# unref: ignore[unused-variable]  # expect: 1 [unused-ignore-comment]
on_next_line = 19  # expect: 1 [unused-variable]


def referenced():  # unref: ignore[unused-function]  # expect: 20 [unused-ignore-comment]
    return 20


class Referenced:
    def referenced_method(self):  # unref: ignore[unused-method]  # expect: 35 [unused-ignore-comment]
        return 21


print(referenced(), Referenced().referenced_method())  # unref: ignore[unused-function]  # expect: 56 [unused-ignore-comment]
visit_name = 22  # unref: ignore[unused-variable]  # expect: 18 [unused-ignore-comment]
__version__ = 23  # unref: ignore[unused-variable]  # expect: 19 [unused-ignore-comment]

# The comment has to be a comment, and spelled this way.
in_string = "# unref: ignore[unused-variable]"  # expect: 1 [unused-variable]
other_tool = 24  # other: ignore[unused-variable]  # expect: 1 [unused-variable]
not_at_start = 25  # A reason. unref: ignore[unused-variable]  # expect: 1 [unused-variable]
