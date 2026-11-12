# A comment must name the codes that it ignores. One that does not ignores nothing, and is reported as
# ty reports an invalid `ty: ignore` comment.
bare = 1  # unref: ignore  # expect: 1 [unused-variable], 11 [invalid-ignore-comment]
empty = 2  # unref: ignore[]  # expect: 1 [unused-variable], 12 [invalid-ignore-comment]
empty_with_spaces = 3  # unref: ignore [ ]  # expect: 1 [unused-variable], 24 [invalid-ignore-comment]
bare_before_other = 4  # unref: ignore  # A reason.  # expect: 1 [unused-variable], 24 [invalid-ignore-comment]
no_whitespace = 5  # unref: ignored  # expect: 1 [unused-variable], 35 [invalid-ignore-comment]
unclosed = 6  # unref: ignore[unused-variable  # expect: 1 [unused-variable], 48 [invalid-ignore-comment]
no_comma = 7  # unref: ignore[unused-variable unused-class]  # expect: 1 [unused-variable], 47 [invalid-ignore-comment]
double_comma = 8  # unref: ignore[unused-variable,,]  # expect: 1 [unused-variable], 51 [invalid-ignore-comment]
not_a_code = 9  # unref: ignore[unused-variable, !]  # expect: 1 [unused-variable], 50 [invalid-ignore-comment]

# A code that is not known is reported, and the other codes of the comment still ignore their findings.
unknown = 10  # unref: ignore[unused-variable, unused-thing]  # expect: 48 [ignore-comment-unknown-rule]
only_unknown = 11  # unref: ignore[Unused_Variable]  # expect: 1 [unused-variable], 36 [ignore-comment-unknown-rule]

# A comment can ignore the findings about the ignore comments of its line, as with ty.
ignored_bare = 12  # unref: ignore  # unref: ignore[unused-variable, invalid-ignore-comment]
ignored_unknown = 13  # unref: ignore[unused-variable, unused-thing, ignore-comment-unknown-rule]
ignored_unused = 14  # unref: ignore[unused-variable, unused-function, unused-ignore-comment]
ignored_unused_in_other = 15  # unref: ignore[unused-variable, unused-function]  # unref: ignore[unused-ignore-comment]
# A comment cannot ignore that it ignores nothing itself.
self_unused = 16  # unref: ignore[unused-variable, unused-ignore-comment]  # expect: 52 [unused-ignore-comment]
only_unused = 17  # unref: ignore[unused-ignore-comment]  # expect: 1 [unused-variable], 19 [unused-ignore-comment]
