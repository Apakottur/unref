import textwrap
from pathlib import Path

import unref


class CheckSetup:
    """
    Utility class for testing `check`.
    """

    def __init__(self, *, root: Path) -> None:
        # Root directory of the project under test.
        self.root = root

    def assert_findings(self, *, files: dict[str, str], findings: list[str]) -> None:
        """
        Write the project files, run the check on the project, and assert the exact findings.

        Args:
            files: The project files, as a mapping of a path relative to the project root to the file
                content. The content is dedented, so it can be indented to match the test.
            findings: The expected findings, in order, each rendered as a string.
        """
        for name, content in files.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(textwrap.dedent(content))

        assert [str(finding) for finding in unref.check([self.root])] == findings
