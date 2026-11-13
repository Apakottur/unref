import io
import re
import tokenize

_MARKER = b"unref:"
_IGNORE = re.compile(r"#\s*unref:\s*ignore\b")


def find_ignore_comments(source: bytes) -> dict[int, int]:
    """
    Find the `# unref: ignore` comments of a Python module.

    Args:
        source: The content of the module.

    Returns:
        The column of the first such comment of each line that has one, by its line. Both start at 1,
        and the column counts characters.
    """
    # Most modules have no such comment, and are not tokenized.
    if _MARKER not in source:
        return {}

    comments: dict[int, int] = {}
    for token in tokenize.tokenize(io.BytesIO(source).readline):
        if token.type == tokenize.COMMENT and (match := _IGNORE.search(token.string)):
            line, column = token.start
            comments.setdefault(line, column + match.start() + 1)
    return comments
