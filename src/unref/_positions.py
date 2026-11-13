import bisect
import importlib.util
import re

# What comes before the name of a function or class, up to the name: the keywords, and the spaces and
# line continuations between them.
_BEFORE_NAME = re.compile(r"(?:async[\s\\]+)?(?:def|class)[\s\\]+")


class SourcePositions:
    """
    Turn the positions in a syntax tree into the lines and columns that a reader sees, as ruff and ty
    report them.

    The syntax tree gives the column of a node as an offset in the UTF-8 bytes of its line, starting at
    0. A column that is reported counts the characters of the line instead, starting at 1.
    """

    def __init__(self, source: bytes) -> None:
        """
        Args:
            source: The content of a module that parses.
        """
        # The source is decoded as Python decodes it, so its lines end with `\n` only.
        self._text = importlib.util.decode_source(source)
        self._starts = [0, *(match.end() for match in re.finditer("\n", self._text))]

    def column(self, line: int, offset: int) -> int:
        """
        Get the column of a position in the syntax tree.

        Args:
            line: The line, starting at 1.
            offset: The offset in the UTF-8 bytes of the line, starting at 0.

        Returns:
            The column, starting at 1.
        """
        start = self._starts[line - 1]
        # Most lines are ASCII, where a character is a byte. Otherwise there are fewer characters
        # before the position than bytes.
        before = self._text[start : start + offset]
        if before.isascii():
            return offset + 1
        return len(before.encode()[:offset].decode()) + 1

    def name(self, line: int, offset: int) -> tuple[int, int]:
        """
        Get the position of the name of a function or class, which ruff and ty point to rather than to
        the `def` or `class` keyword.

        Args:
            line: The line of the function or class, starting at 1.
            offset: The offset of the function or class in the UTF-8 bytes of the line, starting at 0.

        Returns:
            The line and the column of the name, starting at 1. The name is on a later line if a line
            continuation comes before it.
        """
        start = self._starts[line - 1] + self.column(line, offset) - 1
        match = _BEFORE_NAME.match(self._text, start)
        position = match.end() if match else start
        name_line = bisect.bisect_right(self._starts, position)
        return name_line, position - self._starts[name_line - 1] + 1
