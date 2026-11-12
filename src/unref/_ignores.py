import io
import re
import tokenize
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

from unref._models import CODES, Finding

_MARKER = b"unref:"
# The start of an ignore comment, which is the start of a comment, or follows another comment on its line.
_START = re.compile(r"#\s*unref:\s*ignore")
_CODE = re.compile(r"[A-Za-z0-9_-]+")

_UNUSED_IGNORE_COMMENT = "unused-ignore-comment"
_INVALID_IGNORE_COMMENT = "invalid-ignore-comment"
_UNKNOWN_CODE = "ignore-comment-unknown-rule"
_NO_CODES = "expected the codes to ignore, in brackets"


@dataclass(frozen=True)
class IgnoredCode:
    """
    A code that an ignore comment names.

    Attributes:
        code: The code, which may not be known.
        column: The column of the code, starting at 1.
    """

    code: str
    column: int


@dataclass(frozen=True)
class IgnoreComment:
    """
    An `# unref: ignore[...]` comment, which ignores the findings of its line that have one of its codes.

    Attributes:
        line: The line of the comment, starting at 1.
        column: The column of the `#` of the comment, starting at 1.
        text: The comment from `unref:` to its closing bracket.
        codes: The codes that the comment names.
    """

    line: int
    column: int
    text: str
    codes: tuple[IgnoredCode, ...]


class _InvalidCommentError(Exception):
    """
    An ignore comment that does not parse.

    Attributes:
        offset: The offset in the comment where it stops parsing.
        message: What is wrong.
    """

    def __init__(self, offset: int, message: str) -> None:
        super().__init__(message)
        self.offset = offset
        self.message = message


def find_ignore_comments(source: bytes, path: Path) -> tuple[list[IgnoreComment], list[Finding]]:
    """
    Find the `# unref: ignore[...]` comments of a Python module, as ty finds its `# ty: ignore[...]`
    comments.

    A comment names its codes in brackets, separated by commas, with optional spaces and trailing comma:
    `# unref: ignore[unused-function, unused-class]`. It can follow another comment on its line, and be
    followed by one. A comment that does not name any code, or that does not parse, ignores nothing and
    is a finding, as is a code that is not known.

    Args:
        source: The content of the module.
        path: The file of the module, for the findings.

    Returns:
        The comments that parse, and the findings about the comments.
    """
    # Most modules have no such comment, and are not tokenized.
    if _MARKER not in source:
        return [], []

    comments: list[IgnoreComment] = []
    findings: list[Finding] = []
    for token in tokenize.tokenize(io.BytesIO(source).readline):
        if token.type != tokenize.COMMENT:
            continue
        line, start = token.start
        for match in _START.finditer(token.string):
            try:
                codes, end = _parse_codes(token.string, match.start(), match.end())
            except _InvalidCommentError as error:
                findings.append(
                    Finding(
                        path=path,
                        line=line,
                        column=start + error.offset + 1,
                        code=_INVALID_IGNORE_COMMENT,
                        name=error.message,
                    )
                )
                continue

            ignored = tuple(IgnoredCode(code=code, column=start + offset + 1) for code, offset in codes)
            findings.extend(
                Finding(path=path, line=line, column=code.column, code=_UNKNOWN_CODE, name=code.code)
                for code in ignored
                if code.code not in CODES
            )
            comments.append(
                IgnoreComment(
                    line=line,
                    column=start + match.start() + 1,
                    text=token.string[match.start() + 1 : end].lstrip(),
                    codes=ignored,
                )
            )
    return comments, findings


def _parse_codes(comment: str, start: int, offset: int) -> tuple[list[tuple[str, int]], int]:
    """
    Parse the codes of an ignore comment, which follow its `unref: ignore`.

    Args:
        comment: The comment token, which can hold other comments as well.
        start: The offset in the token of the `#` of the ignore comment.
        offset: The offset in the token just after `ignore`.

    Returns:
        The codes, each with its offset in the token, and the offset just after the closing bracket.

    Raises:
        _InvalidCommentError: The comment does not parse, or names no code.
    """
    position = _skip_spaces(comment, offset)
    if position == offset and not comment.startswith(("[", "#"), position) and position < len(comment):
        raise _InvalidCommentError(offset, "no whitespace after `ignore`")
    if not comment.startswith("[", position):
        raise _InvalidCommentError(start, _NO_CODES)

    codes: list[tuple[str, int]] = []
    position += 1
    while not comment.startswith("]", position := _skip_spaces(comment, position)):
        code = _CODE.match(comment, position)
        if code is None:
            raise _InvalidCommentError(position, "expected a code")
        codes.append((code.group(), position))
        position = _skip_spaces(comment, code.end())
        if comment.startswith("]", position):
            break
        if not comment.startswith(",", position):
            raise _InvalidCommentError(position, "expected a comma separating the codes")
        position += 1

    if not codes:
        raise _InvalidCommentError(start, _NO_CODES)
    return codes, position + 1


def _skip_spaces(text: str, position: int) -> int:
    """
    Get the offset of the first character at or after a position that is not a space.
    """
    while position < len(text) and text[position].isspace():
        position += 1
    return position


def ignored_codes(comments: Iterable[IgnoreComment]) -> set[tuple[int, str]]:
    """
    Get the codes that ignore comments ignore.

    Returns:
        Each code with the line that it is ignored on.
    """
    return {(comment.line, code.code) for comment in comments for code in comment.codes}


def apply_ignore_comments(
    path: Path,
    comments: list[IgnoreComment],
    findings: list[Finding],
    *,
    ignored: Iterable[tuple[int, str]],
) -> list[Finding]:
    """
    Apply the ignore comments of a file to its findings about ignore comments, and find the codes of the
    comments that ignore nothing, as ty does.

    A comment that ignores nothing is a finding at its `#`. A code of a comment that ignores other
    findings is a finding at the code. A comment can ignore the findings about the comments of its line,
    including `unused-ignore-comment` for the other codes of its line, but not for itself.

    Args:
        path: The file.
        comments: The ignore comments of the file.
        findings: The findings about the ignore comments of the file, which the comments can ignore.
        ignored: The findings of unused definitions that the comments ignore, each as its line and code.

    Returns:
        The findings about the ignore comments that are left, and one for each code that ignores nothing.
    """
    used: set[tuple[IgnoreComment, int]] = set()

    def ignore(line: int, code: str) -> bool:
        """Mark the last code of the line that ignores a finding as used, and tell whether there is one."""
        matches = [
            (comment, index)
            for comment in comments
            if comment.line == line
            for index, ignored_code in enumerate(comment.codes)
            if ignored_code.code == code
        ]
        if matches:
            used.add(matches[-1])
        return bool(matches)

    for line, code in ignored:
        ignore(line, code)
    left = [finding for finding in findings if not ignore(finding.line, finding.code)]

    # The known codes that ignore nothing. Those other than `unused-ignore-comment` can be ignored by an
    # `unused-ignore-comment` code of their line, which is then used, so they are found first.
    unused = [
        (comment, index)
        for comment in comments
        for index, ignored_code in enumerate(comment.codes)
        if (comment, index) not in used and ignored_code.code in CODES and ignored_code.code != _UNUSED_IGNORE_COMMENT
    ]
    unused = [(comment, index) for comment, index in unused if not ignore(comment.line, _UNUSED_IGNORE_COMMENT)]
    unused += [
        (comment, index)
        for comment in comments
        for index, ignored_code in enumerate(comment.codes)
        if (comment, index) not in used and ignored_code.code == _UNUSED_IGNORE_COMMENT
    ]

    # A comment whose codes all ignore nothing is a finding as a whole.
    for comment in comments:
        indexes = sorted(index for unused_comment, index in unused if unused_comment is comment)
        if indexes and len(indexes) == len(comment.codes):
            left.append(
                Finding(
                    path=path,
                    line=comment.line,
                    column=comment.column,
                    code=_UNUSED_IGNORE_COMMENT,
                    name=comment.text,
                )
            )
        else:
            left.extend(
                Finding(
                    path=path,
                    line=comment.line,
                    column=comment.codes[index].column,
                    code=_UNUSED_IGNORE_COMMENT,
                    name=comment.codes[index].code,
                )
                for index in indexes
            )
    return left
