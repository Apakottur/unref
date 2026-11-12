import io
import re
import tokenize

_MARKER = b"unref:"
_IGNORE = re.compile(r"#\s*unref:\s*ignore\b")


def find_ignored_lines(source: bytes) -> set[int]:
    """
    Find the lines of a Python module that have an `# unref: ignore` comment.

    Args:
        source: The content of the module.

    Returns:
        The lines, starting at 1.
    """
    # Most modules have no such comment, and are not tokenized.
    if _MARKER not in source:
        return set()

    return {
        token.start[0]
        for token in tokenize.tokenize(io.BytesIO(source).readline)
        if token.type == tokenize.COMMENT and _IGNORE.search(token.string)
    }
