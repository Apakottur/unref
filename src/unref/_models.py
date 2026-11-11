from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Finding:
    """
    A definition that nothing references.

    Attributes:
        path: The file that holds the definition.
        line: The line of the definition in the file, starting at 1.
        kind: What was found, for example "unused function".
    """

    path: Path
    line: int
    kind: str
