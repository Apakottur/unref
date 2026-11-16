"""
The files that the tests of the command use: the case projects, and the configuration files that a test
writes.
"""

from pathlib import Path

# The case projects, which the tests of the command run on.
CASES_DIR = Path(__file__).parent.parent / "cases"


def write_config(directory: Path, content: str = "", name: str = "unref.toml") -> Path:
    """
    Write a configuration file.

    Args:
        directory: The directory to write it in.
        content: What it holds, which is no options by default.
        name: Its file name, such as `pyproject.toml` for options under `[tool.unref]`.

    Returns:
        The path of the file.
    """
    config = directory / name
    config.write_text(content)
    return config
